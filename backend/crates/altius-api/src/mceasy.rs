use std::sync::Arc;

use anyhow::Context;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use urlencoding::encode as pct_encode;

use crate::config::MceasyConfig;
use crate::error::ApiError;

/// McEasy VSMS/TMS HTTP client.
///
/// The DTOs are intentionally sparse and use `Option`/`#[serde(default)]` so
/// the parser stays tolerant if McEasy adds or omits fields we don't need.
#[derive(Debug, Clone)]
pub struct MceasyClient {
    http: reqwest::Client,
    config: Arc<MceasyConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MceasyVehicle {
    pub id: Option<String>,
    pub license_plate: Option<String>,
    pub display_name: Option<String>,
    #[serde(alias = "vehicle_id")]
    pub vehicle_id: Option<String>,
    pub driver_name: Option<String>,
    pub driver_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub speed: Option<f64>,
    pub ignition: Option<bool>,
    #[serde(alias = "recorded_at", alias = "gps_time")]
    pub recorded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MceasyDriver {
    pub id: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    #[serde(alias = "driver_id")]
    pub driver_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MceasyPosition {
    pub vehicle_id: Option<String>,
    pub license_plate: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub speed: Option<f64>,
    pub ignition: Option<bool>,
    #[serde(alias = "recorded_at", alias = "gps_time")]
    pub recorded_at: Option<DateTime<Utc>>,
    pub driver_id: Option<String>,
    pub driver_name: Option<String>,
}

impl MceasyClient {
    pub fn new(http: reqwest::Client, config: Arc<MceasyConfig>) -> Self {
        Self { http, config }
    }

    pub fn config(&self) -> &MceasyConfig {
        &self.config
    }

    /// List all vehicles from the VSMS public fleet endpoint.
    ///
    /// Documented at `GET /vehicles/` in the McEasy VSMS v2 public API
    /// (https://vsms-v2-public.mceasy.com/docs/). The response is a paginated
    /// array of vehicle objects; we deserialize tolerantly with `Option` fields.
    pub async fn list_vehicles(&self) -> Result<Vec<MceasyVehicle>, ApiError> {
        self.get("/vehicles/").await
    }

    /// List all drivers from the VSMS public users endpoint.
    ///
    /// Documented at `GET /external/users/{third_party_id}` (single user by
    /// third-party id) and `POST /users` (create). The list-all path is
    /// `GET /external/users/` with an empty third-party id, which the API
    /// treats as "list all". We tolerate a missing/empty response.
    pub async fn list_drivers(&self) -> Result<Vec<MceasyDriver>, ApiError> {
        self.get("/external/users/").await
    }

    /// Get the last known position for a single McEasy vehicle by id.
    ///
    /// Documented at `GET /trips/last/{vehicle_id}` — the API rejects a
    /// missing vehicle id with `E-T1/L1-2 "Vehicle ID is required"`.
    pub async fn vehicle_last_position(
        &self,
        vehicle_id: &str,
    ) -> Result<MceasyPosition, ApiError> {
        self.get(&format!("/trips/last/{}", pct_encode(vehicle_id)))
            .await
    }

    /// Create a temporary live view for a set of license plates.
    /// Returns the view id that can be passed to `get_temp_live_view`.
    ///
    /// Documented at `POST /live-data/create_temp_live_view` — accepts
    /// `licensePlates` (string array) and optional `pickedColumns`. The API
    /// rejects an empty/invalid `licensePlates` with `E-LD1/CTLV1-04`.
    pub async fn create_temp_live_view(&self, plates: &[&str]) -> Result<String, ApiError> {
        let body = serde_json::json!({ "licensePlates": plates });
        let url = format!("{}/live-data/create_temp_live_view", self.config.base_url);
        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|_| ApiError::Unavailable("mceasy live view create failed".into()))?;
        let status = res.status();
        if !status.is_success() {
            return Err(ApiError::Unavailable(format!(
                "mceasy live view create returned {}",
                status.as_u16()
            )));
        }
        let json: serde_json::Value = res.json().await.map_err(|e| ApiError::Internal(e.into()))?;
        json.get("data")
            .and_then(|d| d.get("viewId").or_else(|| d.get("id")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ApiError::Unavailable("mceasy live view id missing".into()))
    }

    /// Get live positions for a set of McEasy vehicle ids under a temporary view.
    ///
    /// Documented at `GET /live-data/temp_live_view` — requires `viewId` and
    /// `vehicleIds` (comma-separated). The API rejects invalid `vehicleIds`
    /// JSON with `E-LD1/GTLV1-06`.
    pub async fn get_temp_live_view(
        &self,
        view_id: &str,
        vehicle_ids: &[&str],
    ) -> Result<Vec<MceasyPosition>, ApiError> {
        let ids = vehicle_ids.join(",");
        let path = format!("/live-data/temp_live_view?viewId={view_id}&vehicleIds={ids}");
        self.get(&path).await
    }

    /// Get offline trip history for a vehicle in a date range.
    ///
    /// Documented at `GET /report/offline-history` — requires a vehicle
    /// identifier (vehicle id, license plate, or IMEI), `start_date`, and
    /// `end_date` in `YYYY-MM-DD`. The API rejects a missing identifier or
    /// date range with `E-R1/OH1-2` and a zero vehicle id with `E-R1/OH1-3`.
    pub async fn offline_history(
        &self,
        vehicle_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MceasyPosition>, ApiError> {
        let start = NaiveDate::from(start.naive_utc());
        let end = NaiveDate::from(end.naive_utc());
        let path = format!(
            "/report/offline-history?vehicle_id={vehicle_id}&start_date={start}&end_date={end}"
        );
        self.get(&path).await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}{}", self.config.base_url, path);
        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(|_| ApiError::Unavailable("mceasy request failed".into()))?;
        let status = res.status();
        if !status.is_success() {
            return Err(ApiError::Unavailable(format!(
                "mceasy returned {}",
                status.as_u16()
            )));
        }
        res.json::<T>()
            .await
            .context("mceasy response parsing")
            .map_err(ApiError::Internal)
    }
}
