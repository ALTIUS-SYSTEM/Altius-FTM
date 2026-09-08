use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::sync::watch;
use tokio::time::interval;
use uuid::Uuid;

use altius_core::gps::{AnomalyFlag, CompareOpts, GeoSample, compare_gps_streams};
use altius_core::{
    Coordinate, DeviceTime, GpsObservation, GpsQuality, GpsReview, GpsReviewClassification,
    GpsReviewReason, GpsSource,
};

use crate::error::ApiError;
use crate::mceasy::{MceasyClient, MceasyDriver, MceasyPosition};
use crate::store::Store;

pub struct MceasyWorker {
    pub client: MceasyClient,
    pub store: Arc<Store>,
}

impl MceasyWorker {
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let poll = self.client.config().poll_seconds;
        let sync = self.client.config().master_sync_seconds;
        let mut poll_tick = interval(Duration::from_secs(poll));
        let mut sync_tick = interval(Duration::from_secs(sync));
        // Run a master sync at startup so the first poll has mappings.
        if let Err(e) = self.sync_all().await {
            tracing::warn!(error = %e, "mceasy master sync failed");
        }
        loop {
            tokio::select! {
                _ = shutdown.changed() => break,
                _ = sync_tick.tick() => {
                    if let Err(e) = self.sync_all().await {
                        tracing::warn!(error = %e, "mceasy master sync failed");
                    }
                }
                _ = poll_tick.tick() => {
                    if let Err(e) = self.poll_all().await {
                        tracing::warn!(error = %e, "mceasy poll failed");
                    }
                }
            }
        }
        tracing::info!("mceasy worker stopped");
    }

    async fn sync_all(&self) -> anyhow::Result<()> {
        let orgs = self.store.organizations().await?;
        for org in orgs {
            if let Err(e) = self.sync_org(&org).await {
                tracing::warn!(org, error = %e, "mceasy org sync failed");
            }
        }
        Ok(())
    }

    pub(crate) async fn sync_org(&self, org: &str) -> anyhow::Result<()> {
        let mceasy_vehicles = self.client.list_vehicles().await?;
        let mceasy_drivers = self.client.list_drivers().await?;

        let vehicles = self.store.monitoring_vehicles(org).await?;
        let users = self.store.users_for_org(org).await?;

        let by_plate: HashMap<String, _> = vehicles
            .iter()
            .filter_map(|v| extract_str(v, "vehicle", "plate").map(|p| (p.to_lowercase(), v)))
            .collect();

        for mv in mceasy_vehicles {
            let Some(plate) = mv.license_plate.as_deref() else {
                continue;
            };
            let Some(mceasy_id) = mv.id.as_deref().or(mv.vehicle_id.as_deref()) else {
                continue;
            };
            if by_plate.contains_key(&plate.to_lowercase()) {
                if let Err(e) = self.store.mceasy_sync_vehicle(org, plate, mceasy_id).await {
                    tracing::warn!(plate, error = %e, "mceasy vehicle sync failed");
                }
            } else {
                tracing::debug!(plate, "mceasy vehicle not matched to local fleet");
            }
        }

        for md in mceasy_drivers {
            if let Some(user_sub) = find_user_for_driver(&md, &users)
                && let Some(mceasy_id) = md.id.as_deref().or(md.driver_id.as_deref())
                && let Err(e) = self
                    .store
                    .mceasy_sync_driver(org, &user_sub, mceasy_id)
                    .await
            {
                tracing::warn!(user_sub, error = %e, "mceasy driver sync failed");
            }
        }

        Ok(())
    }

    async fn poll_all(&self) -> anyhow::Result<()> {
        let orgs = self.store.organizations().await?;
        for org in orgs {
            if let Err(e) = self.poll_org(&org).await {
                tracing::warn!(org, error = %e, "mceasy org poll failed");
            }
        }
        // Prune old observations after each global poll.
        let retention = self.client.config().retention_hours;
        let boundary = Utc::now() - chrono::Duration::hours(retention as i64);
        if let Err(e) = self.store.prune_gps_observations_older_than(boundary).await {
            tracing::warn!(error = %e, "mceasy prune failed");
        }
        Ok(())
    }

    pub(crate) async fn poll_org(&self, org: &str) -> anyhow::Result<()> {
        let vehicles = self.store.monitoring_vehicles(org).await?;
        let mut by_plate: HashMap<String, (String, Option<String>)> = HashMap::new();
        for v in &vehicles {
            let plate = extract_str(v, "vehicle", "plate").unwrap_or_default();
            let mceasy_id = extract_str(v, "vehicle", "mceasy-vehicle-id");
            if let Some(mid) = mceasy_id {
                let hub_id = extract_str(v, "hub", "hub-id").unwrap_or_default();
                by_plate.insert(
                    plate.to_string(),
                    (hub_id.to_string(), Some(mid.to_string())),
                );
            }
        }

        if by_plate.is_empty() {
            return Ok(());
        }

        // Prefer batch live view when we have Mceasy vehicle ids.
        let plates: Vec<&str> = by_plate.keys().map(|s| s.as_str()).collect();
        let vehicle_ids: Vec<&str> = by_plate
            .values()
            .filter_map(|(_, mid)| mid.as_deref())
            .collect();

        let positions = if !vehicle_ids.is_empty() {
            match self.client.create_temp_live_view(&plates).await {
                Ok(view_id) => {
                    self.client
                        .get_temp_live_view(&view_id, &vehicle_ids)
                        .await?
                }
                Err(_) => {
                    // Fallback to per-vehicle last position.
                    fetch_positions_individually(&self.client, &vehicle_ids).await?
                }
            }
        } else {
            Vec::new()
        };

        // Fetch app observations for this org from the last 15 minutes.
        let since = Utc::now() - chrono::Duration::minutes(15);
        let app_obs = self
            .store
            .gps_observations_for_org(org, Some("app_gps"), Some(since))
            .await?;
        let app_samples = observations_to_samples(&app_obs)?;

        for pos in positions {
            let Some(plate) = pos.license_plate.as_deref().or_else(|| {
                by_plate
                    .iter()
                    .find(|(_, (_, mid))| mid.as_deref() == pos.vehicle_id.as_deref())
                    .map(|(p, _)| p.as_str())
            }) else {
                continue;
            };
            let Some((hub_id, _)) = by_plate.get(plate) else {
                continue;
            };

            let driver_id = pos
                .driver_id
                .clone()
                .or(pos.driver_name.clone())
                .unwrap_or_default();
            let recorded_at = pos.recorded_at.unwrap_or_else(Utc::now);

            let obs = GpsObservation {
                id: Uuid::new_v4().to_string(),
                tenant_id: org.to_string(),
                hub_id: hub_id.clone(),
                driver_id: driver_id.clone(),
                vehicle_id: Some(plate.to_string()),
                time: DeviceTime {
                    utc: recorded_at,
                    offset_minutes: 0,
                },
                source: GpsSource::VehicleGps,
                quality: if pos.latitude.is_some() && pos.longitude.is_some() {
                    GpsQuality::Accurate
                } else {
                    GpsQuality::Unavailable
                },
                location: pos
                    .latitude
                    .and_then(|lat| pos.longitude.map(|lng| Coordinate { lat, lng })),
                accuracy_meters: None,
                speed_mps: pos.speed,
                mock_location_reported: None,
            };

            if let Err(e) = self.store.record_gps_observation(&obs).await {
                tracing::warn!(plate, error = %e, "record vehicle gps observation failed");
                continue;
            }

            // Compare this vehicle sample with the app samples for the same driver.
            let vehicle_samples = vec![GeoSample {
                lat: pos.latitude.unwrap_or(f64::NAN),
                lng: pos.longitude.unwrap_or(f64::NAN),
                at: recorded_at.timestamp_millis() as f64,
                accuracy_meters: None,
            }];
            let app_for_driver: Vec<GeoSample> = app_samples
                .iter()
                .filter(|s| s.driver_id == driver_id)
                .map(|s| GeoSample {
                    lat: s.coord.lat,
                    lng: s.coord.lng,
                    at: s.at_ms as f64,
                    accuracy_meters: s.accuracy,
                })
                .collect();

            let result = compare_gps_streams(
                &app_for_driver,
                &vehicle_samples,
                CompareOpts {
                    max_time_gap_ms: 60_000.0,
                    threshold_meters: 150.0,
                },
            );

            let review = GpsReview {
                id: Uuid::new_v4().to_string(),
                tenant_id: org.to_string(),
                hub_id: hub_id.clone(),
                driver_id: driver_id.clone(),
                vehicle_id: Some(plate.to_string()),
                app_observation_id: obs.id.clone(),
                vehicle_observation_id: Some(obs.id),
                classification: match result.flag {
                    AnomalyFlag::None => GpsReviewClassification::Consistent,
                    AnomalyFlag::Review => GpsReviewClassification::ReviewRequired,
                    AnomalyFlag::InsufficientData => GpsReviewClassification::InsufficientData,
                },
                separation_meters: if result.variance_meters > 0 {
                    Some(result.variance_meters as f64)
                } else {
                    None
                },
                time_delta_seconds: None,
                reason: if app_for_driver.is_empty() {
                    GpsReviewReason::MissingPair
                } else {
                    GpsReviewReason::Separation
                },
                reviewed_by: None,
                created_at: Utc::now(),
            };

            if let Err(e) = self.store.record_gps_review(&review).await {
                tracing::warn!(plate, error = %e, "record gps review failed");
            }
        }

        Ok(())
    }
}

struct ParsedObservation {
    driver_id: String,
    coord: Coordinate,
    at_ms: i64,
    accuracy: Option<f64>,
}

fn observations_to_samples(obs: &[Value]) -> anyhow::Result<Vec<ParsedObservation>> {
    let mut out = Vec::new();
    for o in obs {
        let obs = o
            .get("observation")
            .ok_or_else(|| anyhow::anyhow!("missing observation"))?;
        let lat = obs.get("latitude").and_then(Value::as_f64);
        let lng = obs.get("longitude").and_then(Value::as_f64);
        let (lat, lng) = match (lat, lng) {
            (Some(lat), Some(lng)) => (lat, lng),
            _ => continue,
        };
        let at = obs
            .get("recorded-at-utc")
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));
        let at_ms = at.map(|d| d.timestamp_millis()).unwrap_or(0);
        out.push(ParsedObservation {
            driver_id: obs
                .get("user-sub")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            coord: Coordinate { lat, lng },
            at_ms,
            accuracy: obs.get("accuracy-meters").and_then(Value::as_f64),
        });
    }
    Ok(out)
}

async fn fetch_positions_individually(
    client: &MceasyClient,
    vehicle_ids: &[&str],
) -> Result<Vec<MceasyPosition>, ApiError> {
    let mut out = Vec::new();
    for &id in vehicle_ids {
        match client.vehicle_last_position(id).await {
            Ok(p) => out.push(p),
            Err(_) => continue,
        }
    }
    Ok(out)
}

fn find_user_for_driver(md: &MceasyDriver, users: &[Value]) -> Option<String> {
    let target_name = md.name.as_deref().unwrap_or("").to_lowercase();
    let target_email = md.email.as_deref().unwrap_or("").to_lowercase();
    for u in users {
        let user = u.get("user")?;
        let name = user
            .get("display-name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let email = user
            .get("user-sub")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        if !target_name.is_empty() && name == target_name {
            return user
                .get("user-sub")
                .and_then(Value::as_str)
                .map(String::from);
        }
        if !target_email.is_empty() && email == target_email {
            return user
                .get("user-sub")
                .and_then(Value::as_str)
                .map(String::from);
        }
    }
    None
}

fn extract_str<'a>(v: &'a Value, outer: &str, inner: &str) -> Option<&'a str> {
    v.get(outer)?.get(inner)?.as_str()
}
