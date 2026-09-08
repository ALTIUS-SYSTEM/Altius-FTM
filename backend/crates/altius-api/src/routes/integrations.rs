//! Admin routes for machine-to-machine (integration) membership.
//!
//! Keycloak confidential clients are created out-of-band. These handlers only
//! bind the service-account subject to an org (and optional hub) in Postgres —
//! no Keycloak Admin API calls.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, org_of, require_admin, store, valid_name};
use crate::AppState;
use crate::error::{ApiError, ApiResult};

#[derive(serde::Deserialize)]
pub(crate) struct ProvisionIntegrationRequest {
    subject: String,
    display_name: String,
    #[serde(default)]
    hub_id: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v3/integrations",
            get(list_integrations).post(provision_integration),
        )
        .route(
            "/api/v3/integrations/{sub}",
            delete(deprovision_integration),
        )
}

async fn list_integrations(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let rows = store(&s)?
        .integrations_for_org(&org)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        json!({ "data": rows, "meta": { "organization": org } }),
    ))
}

async fn provision_integration(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<ProvisionIntegrationRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let subject = req.subject.trim();
    if subject.is_empty() || subject.chars().count() > 128 {
        return Err(ApiError::BadRequest(
            "subject must be 1-128 characters".into(),
        ));
    }
    let display_name = valid_name(&req.display_name)?;
    let org = org_of(&s, &principal.subject).await?;
    let hub = req
        .hub_id
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty());
    store(&s)?
        .provision_service_account(&org, subject, display_name, hub)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(json!({
        "data": {
            "subject": subject,
            "organization": org,
            "hub": hub,
            "role": "integration",
        }
    })))
}

async fn deprovision_integration(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(subject): Path<String>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let removed = store(&s)?
        .deprovision_service_account(&org, &subject)
        .await
        .map_err(ApiError::Internal)?;
    if !removed {
        return Err(ApiError::NotFound);
    }
    Ok(Json(json!({
        "data": {
            "subject": subject,
            "organization": org,
            "removed": true,
        }
    })))
}
