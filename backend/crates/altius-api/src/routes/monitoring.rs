use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde_json::{Value, json};

use crate::AppState;
use crate::auth::AuthUser;
use crate::error::ApiResult;

use super::{org_of, require_super_admin, store};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/monitoring/vehicles", get(list_monitoring_vehicles))
        .route("/api/v3/monitoring/reviews", get(list_gps_reviews))
        .route(
            "/api/v3/monitoring/reviews/{id}/reviewed",
            post(mark_gps_review_reviewed),
        )
        .route("/api/v3/monitoring/mceasy/sync", post(trigger_mceasy_sync))
        .route("/api/v3/monitoring/mceasy/status", get(mceasy_status))
}

async fn list_monitoring_vehicles(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    require_super_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let since = Utc::now() - chrono::Duration::hours(1);
    let (vehicles, observations, reviews) = tokio::join!(
        store(&s)?.monitoring_vehicles(&org),
        store(&s)?.gps_observations_for_org(&org, None, Some(since)),
        store(&s)?.gps_reviews_for_org(&org),
    );
    let vehicles = vehicles.map_err(crate::error::ApiError::Internal)?;
    let observations = observations.map_err(crate::error::ApiError::Internal)?;
    let reviews = reviews.map_err(crate::error::ApiError::Internal)?;
    Ok(Json(json!({
        "data": {
            "vehicles": vehicles,
            "observations": observations,
            "reviews": reviews,
        },
        "meta": { "organization": org }
    })))
}

async fn list_gps_reviews(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    require_super_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let reviews = store(&s)?
        .gps_reviews_for_org(&org)
        .await
        .map_err(crate::error::ApiError::Internal)?;
    Ok(Json(
        json!({ "data": reviews, "meta": { "organization": org } }),
    ))
}

async fn mark_gps_review_reviewed(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_super_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let changed = store(&s)?
        .mark_gps_review_reviewed(&org, &id, &principal.subject)
        .await
        .map_err(crate::error::ApiError::Internal)?;
    Ok(Json(
        json!({ "data": { "reviewId": id, "reviewed": changed } }),
    ))
}

async fn trigger_mceasy_sync(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    require_super_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let Some(client) = s.mceasy_client.clone() else {
        return Err(crate::error::ApiError::Unavailable(
            "mceasy not configured".into(),
        ));
    };
    let store = Arc::new(store(&s)?.clone());
    tokio::spawn(async move {
        let worker = crate::mceasy_worker::MceasyWorker { client, store };
        if let Err(e) = worker.sync_org(&org).await {
            tracing::warn!(org, error = %e, "manual mceasy sync failed");
        }
        if let Err(e) = worker.poll_org(&org).await {
            tracing::warn!(org, error = %e, "manual mceasy poll failed");
        }
    });
    Ok(Json(json!({ "data": { "syncing": true } })))
}

async fn mceasy_status(State(s): State<Arc<AppState>>) -> ApiResult<Json<Value>> {
    Ok(Json(json!({
        "data": {
            "configured": s.mceasy_client.is_some(),
            "base_url": s.config.mceasy.as_ref().map(|c| &c.base_url),
            "poll_seconds": s.config.mceasy.as_ref().map(|c| c.poll_seconds),
            "master_sync_seconds": s.config.mceasy.as_ref().map(|c| c.master_sync_seconds),
            "retention_hours": s.config.mceasy.as_ref().map(|c| c.retention_hours),
        }
    })))
}
