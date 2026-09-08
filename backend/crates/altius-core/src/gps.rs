//! GPS integrity controls — the Rust half of a two-implementation pair.
//!
//! The web dashboard runs these in TypeScript (`packages/algos`); the McEasy
//! polling worker runs them here, because it lives server-side and cannot call
//! into the TS. Two implementations of a fraud control drift, and the drift is
//! silent: a fail-open bug produces a clean result, not an error.
//!
//! Both sides are pinned by `packages/algos/fixtures/gps-controls.json`. Every
//! case in that file encodes a bug that actually shipped — absence of
//! corroboration reading as clean, one NaN coordinate clearing a batch, a
//! corridor with nothing measurable affirming "inside". Add a case there, not
//! only here.

use serde::{Deserialize, Serialize};

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoSample {
    pub lat: f64,
    pub lng: f64,
    /// Milliseconds since the Unix epoch.
    pub at: f64,
    pub accuracy_meters: Option<f64>,
}

impl GeoSample {
    fn point(&self) -> GeoPoint {
        GeoPoint {
            lat: self.lat,
            lng: self.lng,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnomalyFlag {
    None,
    Review,
    InsufficientData,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnomalyResult {
    pub variance_meters: i64,
    pub matched: usize,
    pub flag: AnomalyFlag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CorridorReason {
    Inside,
    Outside,
    Inaccurate,
    InsufficientCorridor,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CorridorResult {
    pub off_route: bool,
    pub distance_meters: f64,
    pub reason: CorridorReason,
}

#[derive(Debug, Clone, Copy)]
pub struct CompareOpts {
    pub max_time_gap_ms: f64,
    pub threshold_meters: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct CorridorOpts {
    pub radius_meters: f64,
    pub consecutive_breach: usize,
    pub max_accuracy_meters: f64,
}

/// Great-circle distance in metres.
///
/// Returns `INFINITY` for any non-finite input rather than NaN: these distances
/// feed a fraud control, and NaN is false in every comparison, so one unusable
/// coordinate would silently clear an entire batch.
pub fn haversine_meters(a: GeoPoint, b: GeoPoint) -> f64 {
    if ![a.lat, a.lng, b.lat, b.lng].iter().all(|v| v.is_finite()) {
        return f64::INFINITY;
    }
    let rad = std::f64::consts::PI / 180.0;
    let d_lat = (b.lat - a.lat) * rad;
    let d_lng = (b.lng - a.lng) * rad;
    let h = (d_lat / 2.0).sin().powi(2)
        + (a.lat * rad).cos() * (b.lat * rad).cos() * (d_lng / 2.0).sin().powi(2);
    EARTH_RADIUS_METERS * 2.0 * h.sqrt().min(1.0).asin()
}

/// Local planar projection about `origin`, matching the TS `toXY`.
fn to_xy(origin: GeoPoint, p: GeoPoint) -> (f64, f64) {
    (
        haversine_meters(
            origin,
            GeoPoint {
                lat: origin.lat,
                lng: p.lng,
            },
        ),
        haversine_meters(
            origin,
            GeoPoint {
                lat: p.lat,
                lng: origin.lng,
            },
        ),
    )
}

/// Shortest distance from `point` to the corridor polyline.
pub fn distance_to_corridor_meters(point: GeoPoint, corridor: &[GeoPoint]) -> f64 {
    let Some(&origin) = corridor.first() else {
        return f64::INFINITY;
    };
    if corridor.len() == 1 {
        return haversine_meters(point, origin);
    }
    let (px, py) = to_xy(origin, point);
    let mut min = f64::INFINITY;
    for pair in corridor.windows(2) {
        let (ax, ay) = to_xy(origin, pair[0]);
        let (bx, by) = to_xy(origin, pair[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
        };
        min = min.min((px - (ax + t * dx)).hypot(py - (ay + t * dy)));
    }
    min
}

/// Whether the samples show a sustained departure from the corridor.
pub fn evaluate_corridor(
    samples: &[GeoSample],
    corridor: &[GeoPoint],
    opts: CorridorOpts,
) -> CorridorResult {
    if corridor.len() < 2 {
        return CorridorResult {
            off_route: false,
            distance_meters: 0.0,
            reason: CorridorReason::InsufficientCorridor,
        };
    }
    let (mut streak, mut last, mut evaluated) = (0usize, 0.0f64, 0usize);
    for s in samples {
        if s.accuracy_meters
            .is_some_and(|a| a > opts.max_accuracy_meters)
        {
            continue;
        }
        let d = distance_to_corridor_meters(s.point(), corridor);
        // An unmeasurable sample must not reset the breach streak — one
        // position-less reading between two breaches would hide the detour.
        if !d.is_finite() {
            continue;
        }
        evaluated += 1;
        last = d;
        streak = if last > opts.radius_meters {
            streak + 1
        } else {
            0
        };
        if streak >= opts.consecutive_breach {
            return CorridorResult {
                off_route: true,
                distance_meters: last,
                reason: CorridorReason::Outside,
            };
        }
    }
    // "inside" is an affirmative claim. With nothing measurable — an empty
    // batch, or every sample self-reporting an accuracy above the limit — there
    // is no evidence either way, and saying "inside" would let a device opt out
    // of the check by inflating accuracyMeters.
    if evaluated == 0 {
        return CorridorResult {
            off_route: false,
            distance_meters: 0.0,
            reason: CorridorReason::Inaccurate,
        };
    }
    CorridorResult {
        off_route: false,
        distance_meters: last,
        reason: CorridorReason::Inside,
    }
}

/// Compare the driver's app GPS against the vehicle tracker's GPS.
pub fn compare_gps_streams(
    app: &[GeoSample],
    vehicle: &[GeoSample],
    opts: CompareOpts,
) -> AnomalyResult {
    let (mut matched, mut worst, mut unmeasurable) = (0usize, 0.0f64, 0usize);

    // Sort once and advance a single pointer: a nested scan is O(n*m) over two
    // arrays whose size the uploading device chooses, so one accepted upload —
    // not a flood — would determine the cost.
    let mut by_time: Vec<&GeoSample> = vehicle.iter().filter(|v| v.at.is_finite()).collect();
    by_time.sort_by(|x, y| x.at.total_cmp(&y.at));
    let mut sorted_app: Vec<&GeoSample> = app.iter().collect();
    sorted_app.sort_by(|x, y| x.at.total_cmp(&y.at));

    let mut cursor = 0usize;
    for a in &sorted_app {
        while cursor + 1 < by_time.len()
            && (by_time[cursor + 1].at - a.at).abs() <= (by_time[cursor].at - a.at).abs()
        {
            cursor += 1;
        }
        let Some(best) = by_time.get(cursor) else {
            continue;
        };
        if (a.at - best.at).abs() <= opts.max_time_gap_ms {
            matched += 1;
            let d = haversine_meters(a.point(), best.point());
            if d.is_finite() {
                worst = worst.max(d);
            } else {
                unmeasurable += 1;
            }
        }
    }

    // Only a genuinely empty comparison is inconclusive. If the app reported
    // positions and the vehicle corroborated none of them, that is the signal —
    // a spoofing device would otherwise defeat the check simply by withholding
    // the vehicle stream or shifting it past the match window.
    if app.is_empty() {
        return AnomalyResult {
            variance_meters: 0,
            matched: 0,
            flag: AnomalyFlag::InsufficientData,
        };
    }
    if matched == 0 {
        return AnomalyResult {
            variance_meters: 0,
            matched: 0,
            flag: AnomalyFlag::Review,
        };
    }
    if unmeasurable > 0 {
        return AnomalyResult {
            variance_meters: worst.round() as i64,
            matched,
            flag: AnomalyFlag::Review,
        };
    }
    AnomalyResult {
        variance_meters: worst.round() as i64,
        matched,
        flag: if worst > opts.threshold_meters {
            AnomalyFlag::Review
        } else {
            AnomalyFlag::None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// The same file `packages/algos` reads. A case passing on one side and
    /// failing on the other means the two implementations have drifted.
    const FIXTURES: &str = include_str!("../../../../packages/algos/fixtures/gps-controls.json");

    /// `null` in the fixture means "no usable coordinate" — what a
    /// `quality: "unavailable"` observation becomes once mapped.
    fn coord(v: &Value, key: &str) -> f64 {
        v.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN)
    }

    fn sample(v: &Value) -> GeoSample {
        GeoSample {
            lat: coord(v, "lat"),
            lng: coord(v, "lng"),
            at: v.get("at").and_then(Value::as_f64).unwrap_or(f64::NAN),
            accuracy_meters: v.get("accuracyMeters").and_then(Value::as_f64),
        }
    }

    fn point(v: &Value) -> GeoPoint {
        GeoPoint {
            lat: coord(v, "lat"),
            lng: coord(v, "lng"),
        }
    }

    fn samples(v: &Value, key: &str) -> Vec<GeoSample> {
        v.get(key)
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |a| a.iter().map(sample).collect())
    }

    #[test]
    fn compare_matches_the_shared_conformance_fixtures() {
        let f: Value = serde_json::from_str(FIXTURES).expect("fixtures parse");
        let block = &f["compareGpsStreams"];
        let opts = CompareOpts {
            max_time_gap_ms: block["opts"]["maxTimeGapMs"].as_f64().unwrap(),
            threshold_meters: block["opts"]["thresholdMeters"].as_f64().unwrap(),
        };
        for case in block["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let got = compare_gps_streams(&samples(case, "app"), &samples(case, "vehicle"), opts);
            let want_flag = case["expect"]["flag"].as_str().unwrap();
            let got_flag = serde_json::to_value(got.flag).unwrap();
            assert_eq!(got_flag.as_str().unwrap(), want_flag, "{name}: flag");
            assert_eq!(
                got.matched as u64,
                case["expect"]["matched"].as_u64().unwrap(),
                "{name}: matched"
            );
        }
    }

    #[test]
    fn corridor_matches_the_shared_conformance_fixtures() {
        let f: Value = serde_json::from_str(FIXTURES).expect("fixtures parse");
        let block = &f["evaluateCorridor"];
        let opts = CorridorOpts {
            radius_meters: block["opts"]["radiusMeters"].as_f64().unwrap(),
            consecutive_breach: block["opts"]["consecutiveBreach"].as_u64().unwrap() as usize,
            max_accuracy_meters: block["opts"]["maxAccuracyMeters"].as_f64().unwrap(),
        };
        let default_corridor: Vec<GeoPoint> = block["corridor"]
            .as_array()
            .unwrap()
            .iter()
            .map(point)
            .collect();

        for case in block["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let corridor: Vec<GeoPoint> =
                case.get("corridor").and_then(Value::as_array).map_or_else(
                    || default_corridor.clone(),
                    |a| a.iter().map(point).collect(),
                );
            let got = evaluate_corridor(&samples(case, "samples"), &corridor, opts);
            assert_eq!(
                got.off_route,
                case["expect"]["offRoute"].as_bool().unwrap(),
                "{name}: offRoute"
            );
            let got_reason = serde_json::to_value(got.reason).unwrap();
            assert_eq!(
                got_reason.as_str().unwrap(),
                case["expect"]["reason"].as_str().unwrap(),
                "{name}: reason"
            );
        }
    }

    #[test]
    fn haversine_is_about_111km_per_degree_of_latitude() {
        let d = haversine_meters(
            GeoPoint { lat: 0.0, lng: 0.0 },
            GeoPoint { lat: 1.0, lng: 0.0 },
        );
        assert!((d - 111_195.0).abs() < 500.0, "got {d}");
        // Non-finite input must be maximal distance, never NaN.
        assert!(
            haversine_meters(
                GeoPoint {
                    lat: f64::NAN,
                    lng: 0.0
                },
                GeoPoint { lat: 0.0, lng: 0.0 }
            )
            .is_infinite()
        );
    }
}
