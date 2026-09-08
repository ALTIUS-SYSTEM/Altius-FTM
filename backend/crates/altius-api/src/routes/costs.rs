use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, org_of, require_staff, store};
use crate::AppState;
use crate::error::{ApiError, ApiResult};
use altius_core::{CostEntry, DailyReport, LhsStatus, VehicleCheck, new_id};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/costs", post(record_cost).get(list_costs))
        .route("/api/v3/reports", post(record_report).get(list_reports))
        .route(
            "/api/v3/vehicle-checks",
            post(record_vehicle_check).get(list_vehicle_checks),
        )
}

async fn record_cost(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<CostEntry>,
) -> ApiResult<Json<Value>> {
    // Mirrors the Zod MoneySchema bound (non-negative, capped, 3-letter
    // currency) that the Rust struct itself no longer enforces at the type
    // level now that amount_minor stays i64 for wire compatibility.
    req.validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    store(&s)?
        .record_cost(&req, &principal.subject)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": { "id": req.id } })))
}

async fn list_costs(State(s): State<Arc<AppState>>, principal: AuthUser) -> ApiResult<Json<Value>> {
    let day = None::<&str>;
    let costs = store(&s)?
        .costs_for_driver(&principal.subject, day)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": costs })))
}

async fn record_report(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(mut req): Json<DailyReport>,
) -> ApiResult<Json<Value>> {
    // Submitting a report never approves it. `status` and `revision` are
    // server-owned; accepting them from the body let a driver post their own
    // LHS as `approved` and skip supervisor review entirely.
    req.status = LhsStatus::Submitted;
    req.revision = 0;
    req.driver_id = principal.subject.clone();
    let (_, hub) = store(&s)?
        .organization_and_hub_of(&principal.subject)
        .await
        .map_err(ApiError::Internal)?
        .unwrap_or_default();
    req.hub_id = hub;
    store(&s)?
        .record_daily_report(&req, &principal.subject)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": { "day": req.day } })))
}

async fn list_reports(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    let reports = store(&s)?
        .reports_for_driver(&principal.subject)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": reports })))
}

async fn record_vehicle_check(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(mut req): Json<VehicleCheck>,
) -> ApiResult<Json<Value>> {
    // A driver submits their own checklist. Scope comes from their token, not
    // the body — the request should never carry another org's tenant id.
    let (org, hub) = store(&s)?
        .organization_and_hub_of(&principal.subject)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Forbidden)?;
    req.id = new_id();
    req.driver_id = principal.subject.clone();
    req.tenant_id = org.clone();
    req.hub_id = hub.clone();
    store(&s)?
        .record_vehicle_check(&org, &hub, &req)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": { "id": req.id } })))
}

async fn list_vehicle_checks(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    require_staff(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let checks = store(&s)?
        .vehicle_checks_for_org(&org, None)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": checks })))
}
