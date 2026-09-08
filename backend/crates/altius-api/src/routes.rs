//! REST surface — `/api/v3` shape from Docs/ALTIUS_API_REFERENCE.md.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::AppState;
use crate::error::{ApiError, ApiResult};
use altius_core::Role;

mod monitoring;

pub(crate) mod agent;
pub(crate) mod auth;
pub(crate) mod costs;
pub(crate) mod hubs;
pub(crate) mod maps;
pub(crate) mod notify;
pub(crate) mod tasks;
pub(crate) mod teams;
pub(crate) mod users;

#[cfg(test)]
mod agent_tool_tests;
#[cfg(test)]
mod crud_tests;

pub(crate) use crate::auth::AuthUser;

pub(crate) fn store(s: &Arc<AppState>) -> ApiResult<&crate::store::Store> {
    s.store
        .as_ref()
        .ok_or_else(|| ApiError::Unavailable("persistence not configured".into()))
}

/// Resolve the caller's org from TypeDB membership only.
///
/// The `organization_id` token claim is deliberately NOT trusted as a fallback:
/// it is a Keycloak user attribute, and if it is self-editable (or minted by
/// another client in the same realm) it becomes a tenant selector the caller
/// controls. A subject with no membership row is forbidden, not free to pick.
pub(crate) async fn org_of(s: &Arc<AppState>, subject: &str) -> ApiResult<String> {
    store(s)?
        .organization_of(subject)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Forbidden)
}

/// True when the principal carries a staff role. Read as a predicate — the
/// `require_*` gates below build on it.
pub(crate) fn is_staff(principal: &AuthUser) -> bool {
    principal.has_role(Role::Admin)
        || principal.has_role(Role::Supervisor)
        || principal.has_role(Role::Lead)
}

/// Gate for org-wide rosters and planning data. A Driver sees their own work,
/// not the organization's user list, hub list or driver list.
pub(crate) fn require_staff(principal: &AuthUser) -> ApiResult<()> {
    if is_staff(principal) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

/// Staff plus `integration` service accounts — org-scoped **read** routes a
/// machine caller may use. Write routes keep `require_staff`.
pub(crate) fn require_staff_or_integration(principal: &AuthUser) -> ApiResult<()> {
    if is_staff(principal) || principal.has_role(Role::Integration) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

/// Gate for operations that only a super-admin may perform.
pub(crate) fn require_super_admin(principal: &AuthUser) -> ApiResult<()> {
    if principal.has_role(Role::SuperAdmin) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

/// Admin-only: create a realm user and bind them to the caller's org and hub.
pub(crate) fn require_admin(principal: &AuthUser) -> ApiResult<()> {
    if principal.has_role(Role::Admin) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

/// Map "the scoped query matched nothing" onto 404 rather than a silent 200.
pub(crate) fn touched_or_404(touched: bool) -> ApiResult<Json<Value>> {
    if touched {
        Ok(Json(json!({ "data": { "ok": true } })))
    } else {
        Err(ApiError::NotFound)
    }
}

pub(crate) fn valid_name(name: &str) -> ApiResult<&str> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 200 {
        return Err(ApiError::BadRequest("name must be 1-200 characters".into()));
    }
    Ok(trimmed)
}

pub(crate) fn valid_coordinate(lat: f64, lng: f64) -> ApiResult<()> {
    if !lat.is_finite()
        || !lng.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lng)
    {
        return Err(ApiError::BadRequest(
            "latitude/longitude out of range".into(),
        ));
    }
    Ok(())
}

async fn health(State(s): State<Arc<AppState>>) -> Json<Value> {
    let persistence = if let Some(st) = &s.store {
        st.ping().await
    } else {
        false
    };
    // Liveness only. Which third-party integrations are configured, and which
    // routing backend is in use, is reconnaissance for an unauthenticated
    // caller — it belongs in logs and an admin-gated diagnostics route.
    tracing::debug!(
        persistence,
        maps = s.config.google_maps_api_key.is_some(),
        maps_mode = ?s.config.google_route_mode,
        agent = s.config.openrouter_api_key.is_some(),
        "health probe"
    );
    Json(json!({
        "status": if persistence { "ok" } else { "degraded" },
        "service": "altius-api",
    }))
}

async fn ready(State(s): State<Arc<AppState>>) -> ApiResult<Json<Value>> {
    let persistence = if let Some(st) = &s.store {
        st.ping().await
    } else {
        false
    };
    if persistence {
        Ok(Json(
            json!({"status": "ok", "service": "altius-api", "ready": true }),
        ))
    } else {
        Err(ApiError::Unavailable("persistence not ready".into()))
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/health", get(health))
        .route("/api/v3/ready", get(ready))
        .merge(auth::router())
        .merge(tasks::router())
        .merge(users::router())
        .merge(teams::router())
        .merge(hubs::router())
        .merge(costs::router())
        .merge(notify::router())
        .merge(agent::router())
        .merge(maps::router())
        .merge(monitoring::router())
}
