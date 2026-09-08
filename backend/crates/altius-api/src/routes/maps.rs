use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use super::AuthUser;
use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::maps::{MAX_OPTIMIZE_WAYPOINTS, MapsClient, mode_marker, schematic_eta_seconds};
use altius_core::Coordinate;

#[derive(serde::Deserialize)]
pub(crate) struct EtaRequest {
    from: Coordinate,
    to: Coordinate,
}

#[derive(serde::Deserialize)]
pub(crate) struct StaticMapRequest {
    markers: Vec<Coordinate>,
    #[serde(default)]
    polyline: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(serde::Deserialize)]
pub(crate) struct GeocodeRequest {
    address: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct AutocompleteRequest {
    input: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/route/eta", post(eta))
        .route("/api/v3/route/geocode", post(geocode))
        .route("/api/v3/route/static-map", post(static_map))
        .route("/api/v3/places/autocomplete", post(autocomplete))
}

async fn eta(
    State(s): State<Arc<AppState>>,
    _principal: AuthUser,
    Json(req): Json<EtaRequest>,
) -> ApiResult<Json<Value>> {
    let maps = MapsClient::new(s.http.clone(), s.config.google_maps_api_key.clone());
    let (seconds, marker) = if maps.enabled() {
        (maps.eta_seconds(req.from, req.to).await?, mode_marker(true))
    } else {
        (schematic_eta_seconds(req.from, req.to), mode_marker(false))
    };
    Ok(Json(
        json!({ "data": { "eta_seconds": seconds }, "meta": marker }),
    ))
}

/// Render a route as a PNG. The Maps key stays server-side: the browser gets
/// an image from *our* origin, never a Google URL bearing the credential.
async fn static_map(
    State(s): State<Arc<AppState>>,
    _principal: AuthUser,
    Json(req): Json<StaticMapRequest>,
) -> ApiResult<axum::response::Response> {
    if req.markers.is_empty() {
        return Err(ApiError::BadRequest("at least one marker required".into()));
    }
    if req.markers.len() > MAX_OPTIMIZE_WAYPOINTS + 1 {
        return Err(ApiError::BadRequest(format!(
            "at most {} markers",
            MAX_OPTIMIZE_WAYPOINTS + 1
        )));
    }
    let maps = MapsClient::new(s.http.clone(), s.config.google_maps_api_key.clone());
    if !maps.enabled() {
        return Err(ApiError::Unavailable("maps not configured".into()));
    }
    let png = maps
        .static_map(
            &req.markers,
            req.polyline.as_deref(),
            req.width.unwrap_or(640),
            req.height.unwrap_or(360),
        )
        .await?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            // Per-caller route geometry: cacheable in the browser, never shared.
            (axum::http::header::CACHE_CONTROL, "private, max-age=300"),
        ],
        png,
    )
        .into_response())
}

async fn geocode(
    State(s): State<Arc<AppState>>,
    _principal: AuthUser,
    Json(req): Json<GeocodeRequest>,
) -> ApiResult<Json<Value>> {
    let maps = MapsClient::new(s.http.clone(), s.config.google_maps_api_key.clone());
    let point = maps.geocode(&req.address).await?;
    Ok(Json(
        json!({ "data": { "location": point }, "meta": mode_marker(maps.enabled()) }),
    ))
}

async fn autocomplete(
    State(s): State<Arc<AppState>>,
    _principal: AuthUser,
    Json(req): Json<AutocompleteRequest>,
) -> ApiResult<Json<Value>> {
    let maps = MapsClient::new(s.http.clone(), s.config.google_maps_api_key.clone());
    let suggestions = maps.autocomplete(&req.input).await?;
    Ok(Json(json!({
        "data": suggestions,
        "meta": mode_marker(maps.enabled())
    })))
}
