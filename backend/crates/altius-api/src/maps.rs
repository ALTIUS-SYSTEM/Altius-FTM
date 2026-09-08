//! Google Maps Platform client — server-side only (key never ships to clients).
//!
//! Operational behavior ported from `googlemaps/google-maps-services-java`:
//! client-side QPS pacing (50 req/s default) and bounded retry on transient
//! 429/5xx responses. Waypoint optimization uses the Directions API
//! `optimize:true` contract (the approach `MicLieg/ShortestRouteFinder` uses),
//! with an offline greedy nearest-neighbor fallback.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

use crate::error::ApiError;
use altius_core::Coordinate;

const DIRECTIONS_URL: &str = "https://maps.googleapis.com/maps/api/directions/json";
const MATRIX_URL: &str = "https://maps.googleapis.com/maps/api/distancematrix/json";
const GEOCODE_URL: &str = "https://maps.googleapis.com/maps/api/geocode/json";
const PLACES_URL: &str = "https://maps.googleapis.com/maps/api/place/autocomplete/json";
const STATIC_MAP_URL: &str = "https://maps.googleapis.com/maps/api/staticmap";

/// Largest static-map image we will render, in pixels per side.
const MAX_STATIC_MAP_PX: u32 = 1280;
/// Refuse to relay anything larger than a plausible map image.
const MAX_STATIC_MAP_BYTES: usize = 4 * 1024 * 1024;

/// Google enforces ~50 QPS per key on the web services — pace accordingly.
const MIN_INTERVAL: Duration = Duration::from_millis(20);
const MAX_ATTEMPTS: u32 = 3;
/// Directions `optimize:true` accepts at most 25 waypoints (+ origin/dest).
pub const MAX_OPTIMIZE_WAYPOINTS: usize = 25;

#[derive(Clone)]
pub struct MapsClient {
    http: reqwest::Client,
    key: Option<String>,
    /// Client-side pacing so a burst never exceeds the per-key QPS budget.
    last_call: Arc<Mutex<Instant>>,
}

#[derive(Debug, Deserialize)]
struct DirectionsResponse {
    status: String,
    #[serde(default)]
    routes: Vec<Route>,
}

#[derive(Debug, Deserialize)]
struct Route {
    legs: Vec<Leg>,
    #[serde(default)]
    waypoint_order: Vec<usize>,
    #[serde(default)]
    overview_polyline: Option<EncodedPolyline>,
}

#[derive(Debug, Deserialize)]
struct EncodedPolyline {
    points: String,
}

#[derive(Debug, Deserialize)]
struct Leg {
    duration: Value,
    distance: Value,
}

#[derive(Debug, Deserialize)]
struct Value {
    value: i64,
}

#[derive(Debug, Deserialize)]
struct GeocodeResponse {
    status: String,
    #[serde(default)]
    results: Vec<GeocodeResult>,
}

#[derive(Debug, Deserialize)]
struct GeocodeResult {
    geometry: Geometry,
}

#[derive(Debug, Deserialize)]
struct Geometry {
    location: LatLng,
}

#[derive(Debug, Deserialize)]
struct LatLng {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Deserialize)]
struct MatrixResponse {
    status: String,
    #[serde(default)]
    rows: Vec<MatrixRow>,
}

#[derive(Debug, Deserialize)]
struct MatrixRow {
    #[serde(default)]
    elements: Vec<MatrixElement>,
}

#[derive(Debug, Deserialize)]
struct MatrixElement {
    status: String,
    #[serde(default)]
    duration: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AutocompleteResponse {
    status: String,
    #[serde(default)]
    predictions: Vec<AutocompletePrediction>,
}

#[derive(Debug, Deserialize)]
struct AutocompletePrediction {
    description: String,
    #[serde(default)]
    place_id: String,
}

/// Ordered result of a stop-optimization call.
#[derive(Debug, serde::Serialize)]
pub struct OptimizedRoute {
    /// Indices into the caller's waypoint list, in visit order.
    pub order: Vec<usize>,
    /// Per-leg drive time in seconds, `order.len()` entries.
    pub leg_seconds: Vec<i64>,
    /// Total route distance in meters.
    pub total_meters: i64,
    /// `"live"` (Directions `optimize:true`) or `"nearest"` (offline fallback).
    pub source: &'static str,
    /// Google-encoded road geometry for the whole route, when live. Renderable
    /// directly by the static-map proxy; `None` in offline/fallback mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polyline: Option<String>,
}

impl MapsClient {
    pub fn new(http: reqwest::Client, key: Option<String>) -> Self {
        Self {
            http,
            key,
            last_call: Arc::new(Mutex::new(Instant::now() - MIN_INTERVAL)),
        }
    }

    pub fn enabled(&self) -> bool {
        self.key.is_some()
    }

    fn key(&self) -> Result<&str, ApiError> {
        self.key
            .as_deref()
            .ok_or_else(|| ApiError::Unavailable("GOOGLE_MAPS_API_KEY not configured".into()))
    }

    /// Pace outgoing requests to the QPS budget shared by every caller.
    async fn pace(&self) {
        let wait = {
            let last = self.last_call.lock().await;
            MIN_INTERVAL.saturating_sub(last.elapsed())
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        *self.last_call.lock().await = Instant::now();
    }

    /// GET with bounded retry on 429/5xx (the Java client's retry contract).
    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> Result<T, ApiError> {
        let mut delay = Duration::from_millis(150);
        for attempt in 0..MAX_ATTEMPTS {
            self.pace().await;
            let res = self
                .http
                .get(url)
                .query(query)
                .send()
                .await
                .map_err(|e| ApiError::upstream("maps", e))?;
            let status = res.status();
            if status.is_success() {
                return res.json().await.map_err(|e| ApiError::upstream("maps", e));
            }
            let retriable = status.as_u16() == 429 || status.is_server_error();
            if !retriable || attempt + 1 == MAX_ATTEMPTS {
                return Err(ApiError::Unavailable(format!("maps http {status}")));
            }
            tokio::time::sleep(delay).await;
            delay *= 2;
        }
        unreachable!()
    }

    /// Road ETA in seconds between two points (driving).
    pub async fn eta_seconds(&self, from: Coordinate, to: Coordinate) -> Result<i64, ApiError> {
        let res: DirectionsResponse = self
            .get(
                DIRECTIONS_URL,
                &[
                    ("origin", format!("{},{}", from.lat, from.lng)),
                    ("destination", format!("{},{}", to.lat, to.lng)),
                    ("key", self.key()?.to_string()),
                ],
            )
            .await?;
        if res.status != "OK" {
            return Err(ApiError::Unavailable(format!("directions: {}", res.status)));
        }
        res.routes
            .first()
            .and_then(|r| r.legs.first())
            .map(|l| l.duration.value)
            .ok_or(ApiError::NotFound)
    }

    /// Order `waypoints` into the shortest route starting at `origin`.
    ///
    /// Live mode calls Directions with `optimize:true`; offline falls back to
    /// greedy nearest-neighbor on straight-line distance.
    pub async fn optimize_stops(
        &self,
        origin: Coordinate,
        waypoints: &[Coordinate],
    ) -> Result<OptimizedRoute, ApiError> {
        if waypoints.is_empty() {
            return Ok(OptimizedRoute {
                order: vec![],
                leg_seconds: vec![],
                total_meters: 0,
                source: if self.enabled() { "live" } else { "nearest" },
                polyline: None,
            });
        }
        if !self.enabled() {
            return Ok(greedy_order(origin, waypoints));
        }
        if waypoints.len() > MAX_OPTIMIZE_WAYPOINTS {
            return Err(ApiError::BadRequest(format!(
                "at most {MAX_OPTIMIZE_WAYPOINTS} waypoints per optimize call"
            )));
        }
        let wp = waypoints
            .iter()
            .map(|p| format!("{},{}", p.lat, p.lng))
            .collect::<Vec<_>>()
            .join("|");
        let dest = waypoints
            .last()
            .map(|p| format!("{},{}", p.lat, p.lng))
            .unwrap();
        let res: DirectionsResponse = self
            .get(
                DIRECTIONS_URL,
                &[
                    ("origin", format!("{},{}", origin.lat, origin.lng)),
                    ("destination", dest),
                    ("waypoints", format!("optimize:true|{wp}")),
                    ("key", self.key()?.to_string()),
                ],
            )
            .await?;
        if res.status != "OK" {
            return Err(ApiError::Unavailable(format!("directions: {}", res.status)));
        }
        let route = res.routes.first().ok_or(ApiError::NotFound)?;
        Ok(OptimizedRoute {
            // Documented to callers as indices into *their* waypoint list, so
            // validate that claim before passing it on: a hostile or changed
            // upstream response would otherwise hand a client an out-of-range
            // index to dereference against its own array.
            order: {
                let o = &route.waypoint_order;
                let in_range = o.iter().all(|&i| i < waypoints.len());
                let unique = o.iter().collect::<std::collections::HashSet<_>>().len() == o.len();
                if o.len() == waypoints.len() && in_range && unique {
                    o.clone()
                } else {
                    tracing::warn!(
                        got = o.len(),
                        want = waypoints.len(),
                        "maps returned an unusable waypoint_order; falling back to input order"
                    );
                    (0..waypoints.len()).collect()
                }
            },
            leg_seconds: route.legs.iter().map(|l| l.duration.value).collect(),
            total_meters: route.legs.iter().map(|l| l.distance.value).sum(),
            source: "live",
            polyline: route.overview_polyline.as_ref().map(|p| p.points.clone()),
        })
    }

    /// Render a route as a PNG via the Static Maps API.
    ///
    /// Proxied rather than handed to the browser as a URL: the API key is a
    /// server-side secret (see the module header), and a client-side map would
    /// require shipping a second, separately-restricted browser key. Returns
    /// the raw image bytes for the caller to relay.
    pub async fn static_map(
        &self,
        markers: &[Coordinate],
        polyline: Option<&str>,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, ApiError> {
        let mut query = static_map_query(markers, polyline, width, height);
        query.push(("key", self.key()?.to_string()));

        self.pace().await;
        let res = self
            .http
            .get(STATIC_MAP_URL)
            .query(&query)
            .send()
            .await
            .map_err(|e| ApiError::upstream("maps", e))?;
        if !res.status().is_success() {
            return Err(ApiError::Unavailable(format!(
                "staticmap http {}",
                res.status()
            )));
        }
        let bytes = res
            .bytes()
            .await
            .map_err(|e| ApiError::upstream("maps", e))?;
        if bytes.len() > MAX_STATIC_MAP_BYTES {
            return Err(ApiError::Unavailable("staticmap response too large".into()));
        }
        Ok(bytes.to_vec())
    }

    /// Stop-pair durations for the routing solver (Distance Matrix).
    pub async fn distance_matrix_seconds(
        &self,
        origins: &[Coordinate],
        destinations: &[Coordinate],
    ) -> Result<Vec<Vec<Option<i64>>>, ApiError> {
        let encode = |pts: &[Coordinate]| {
            pts.iter()
                .map(|p| format!("{},{}", p.lat, p.lng))
                .collect::<Vec<_>>()
                .join("|")
        };
        let res: MatrixResponse = self
            .get(
                MATRIX_URL,
                &[
                    ("origins", encode(origins)),
                    ("destinations", encode(destinations)),
                    ("key", self.key()?.to_string()),
                ],
            )
            .await?;
        if res.status != "OK" {
            return Err(ApiError::Unavailable(format!("matrix: {}", res.status)));
        }
        Ok(res
            .rows
            .iter()
            .map(|row| {
                row.elements
                    .iter()
                    .map(|el| {
                        if el.status == "OK" {
                            el.duration.as_ref().map(|d| d.value)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .collect())
    }

    /// Address text → list of autocomplete suggestions.
    pub async fn autocomplete(&self, input: &str) -> Result<Vec<serde_json::Value>, ApiError> {
        let res: AutocompleteResponse = self
            .get(
                PLACES_URL,
                &[
                    ("input", input.to_string()),
                    ("key", self.key()?.to_string()),
                ],
            )
            .await?;
        if res.status == "ZERO_RESULTS" {
            return Ok(vec![]);
        }
        if res.status != "OK" {
            return Err(ApiError::Unavailable(format!("places: {}", res.status)));
        }
        Ok(res
            .predictions
            .into_iter()
            .map(|p| {
                json!({
                    "description": p.description,
                    "placeId": p.place_id,
                })
            })
            .collect())
    }

    /// Address → coordinate; `None` when unresolvable.
    pub async fn geocode(&self, address: &str) -> Result<Option<Coordinate>, ApiError> {
        let res: GeocodeResponse = self
            .get(
                GEOCODE_URL,
                &[
                    ("address", address.to_string()),
                    ("key", self.key()?.to_string()),
                ],
            )
            .await?;
        if res.status == "ZERO_RESULTS" {
            return Ok(None);
        }
        if res.status != "OK" {
            return Err(ApiError::Unavailable(format!("geocode: {}", res.status)));
        }
        Ok(res.results.first().map(|r| Coordinate {
            lat: r.geometry.location.lat,
            lng: r.geometry.location.lng,
        }))
    }
}

/// Offline fallback: greedy nearest-neighbor ordering on haversine distance.
/// Build the Static Maps query minus the credential.
///
/// Split out so the size clamp and marker cap are testable without a network
/// call — and so the key is appended in exactly one place, by the caller that
/// holds it, rather than threaded through the formatting logic.
fn static_map_query(
    markers: &[Coordinate],
    polyline: Option<&str>,
    width: u32,
    height: u32,
) -> Vec<(&'static str, String)> {
    let w = width.clamp(1, MAX_STATIC_MAP_PX);
    let h = height.clamp(1, MAX_STATIC_MAP_PX);
    let mut query = vec![
        ("size", format!("{w}x{h}")),
        ("scale", "2".to_string()),
        ("format", "png".to_string()),
    ];
    // Numbered pins in visit order, so the image alone tells the sequence.
    for (i, m) in markers.iter().take(MAX_OPTIMIZE_WAYPOINTS + 1).enumerate() {
        query.push((
            "markers",
            format!("color:0x0FA3B1|label:{}|{},{}", (i % 9) + 1, m.lat, m.lng),
        ));
    }
    if let Some(p) = polyline {
        query.push(("path", format!("weight:4|color:0x0FA3B1CC|enc:{p}")));
    } else if markers.len() > 1 {
        // No road geometry (offline solver): draw the stop order directly
        // rather than implying a road-following path we do not have.
        let pts = markers
            .iter()
            .map(|m| format!("{},{}", m.lat, m.lng))
            .collect::<Vec<_>>()
            .join("|");
        query.push(("path", format!("weight:3|color:0x94A3B8AA|{pts}")));
    }
    query
}

pub fn greedy_order(origin: Coordinate, waypoints: &[Coordinate]) -> OptimizedRoute {
    let mut remaining: Vec<usize> = (0..waypoints.len()).collect();
    let mut order = Vec::with_capacity(waypoints.len());
    let mut leg_seconds = Vec::with_capacity(waypoints.len());
    let mut total_meters = 0i64;
    let mut here = origin;
    while !remaining.is_empty() {
        let (pos, &idx) = remaining
            .iter()
            .enumerate()
            .min_by(|a, b| {
                haversine_meters(here, waypoints[*a.1])
                    .total_cmp(&haversine_meters(here, waypoints[*b.1]))
            })
            .unwrap();
        let meters = haversine_meters(here, waypoints[idx]);
        total_meters += meters as i64;
        leg_seconds.push(schematic_eta_seconds(here, waypoints[idx]));
        here = waypoints[idx];
        order.push(idx);
        remaining.remove(pos);
    }
    OptimizedRoute {
        order,
        leg_seconds,
        total_meters,
        source: "nearest",
        // Straight-line fallback has no road geometry; the client draws the
        // stop order itself rather than being handed a fake path.
        polyline: None,
    }
}

fn haversine_meters(a: Coordinate, b: Coordinate) -> f64 {
    const R: f64 = 6_371_000.0;
    let d_lat = (b.lat - a.lat).to_radians();
    let d_lng = (b.lng - a.lng).to_radians();
    let h = (d_lat / 2.0).sin().powi(2)
        + a.lat.to_radians().cos() * b.lat.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().asin()
}

/// Fallback ETA used when no Maps key is configured (straight-line, 30 km/h).
pub fn schematic_eta_seconds(from: Coordinate, to: Coordinate) -> i64 {
    (haversine_meters(from, to) / (30_000.0 / 3600.0)) as i64
}

/// Marker payload clients use to render "demo" vs "live" routing.
pub fn mode_marker(enabled: bool) -> serde_json::Value {
    json!({ "maps": if enabled { "live" } else { "demo" } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_map_query_clamps_and_never_carries_the_key() {
        let pts = vec![
            Coordinate {
                lat: -6.2,
                lng: 106.8
            };
            MAX_OPTIMIZE_WAYPOINTS + 10
        ];
        let q = static_map_query(&pts, Some("abc123"), 99_999, 0);

        let size = &q.iter().find(|(k, _)| *k == "size").unwrap().1;
        assert_eq!(
            size,
            &format!("{MAX_STATIC_MAP_PX}x1"),
            "size must clamp both ways"
        );

        let markers = q.iter().filter(|(k, _)| *k == "markers").count();
        assert_eq!(
            markers,
            MAX_OPTIMIZE_WAYPOINTS + 1,
            "marker count must be capped"
        );

        let path = &q.iter().find(|(k, _)| *k == "path").unwrap().1;
        assert!(
            path.contains("enc:abc123"),
            "live geometry should be sent encoded"
        );

        // The credential is appended by the caller that holds it, never here.
        assert!(!q.iter().any(|(k, _)| *k == "key"));
    }

    #[test]
    fn static_map_falls_back_to_stop_order_without_geometry() {
        let pts = [
            Coordinate { lat: 1.0, lng: 2.0 },
            Coordinate { lat: 3.0, lng: 4.0 },
        ];
        let q = static_map_query(&pts, None, 640, 360);
        let path = &q.iter().find(|(k, _)| *k == "path").unwrap().1;
        // Straight segments between stops — never an "enc:" path we do not have.
        assert!(path.contains("1,2|3,4"));
        assert!(!path.contains("enc:"));

        // A single marker has no path at all.
        assert!(
            !static_map_query(&pts[..1], None, 640, 360)
                .iter()
                .any(|(k, _)| *k == "path")
        );
    }

    #[test]
    fn greedy_orders_nearest_first() {
        let origin = Coordinate { lat: 0.0, lng: 0.0 };
        // Farthest point listed first to prove ordering isn't input order.
        let wps = [
            Coordinate {
                lat: 0.0,
                lng: 0.20,
            },
            Coordinate {
                lat: 0.0,
                lng: 0.05,
            },
            Coordinate {
                lat: 0.0,
                lng: 0.10,
            },
        ];
        let out = greedy_order(origin, &wps);
        assert_eq!(out.order, vec![1, 2, 0]);
        assert_eq!(out.leg_seconds.len(), 3);
        assert!(out.total_meters > 0);
        assert_eq!(out.source, "nearest");
    }

    #[tokio::test]
    async fn optimize_offline_uses_fallback() {
        let client = MapsClient::new(reqwest::Client::new(), None);
        let out = client
            .optimize_stops(
                Coordinate {
                    lat: -6.2,
                    lng: 106.8,
                },
                &[
                    Coordinate {
                        lat: -6.3,
                        lng: 106.9,
                    },
                    Coordinate {
                        lat: -6.21,
                        lng: 106.81,
                    },
                ],
            )
            .await
            .unwrap();
        assert_eq!(out.source, "nearest");
        assert_eq!(out.order.first(), Some(&1));
    }

    #[tokio::test]
    async fn optimize_rejects_too_many_waypoints() {
        let client = MapsClient::new(reqwest::Client::new(), Some("k".into()));
        let wps = vec![Coordinate { lat: 0.0, lng: 0.0 }; 26];
        let err = client
            .optimize_stops(Coordinate { lat: 0.0, lng: 0.0 }, &wps)
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }
}
