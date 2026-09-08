use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{
    AuthUser, org_of, require_admin, require_staff, store, touched_or_404, valid_coordinate,
    valid_name,
};
use crate::AppState;
use crate::error::{ApiError, ApiResult};
use altius_core::new_id;

#[derive(serde::Deserialize)]
pub(crate) struct HubRequest {
    #[serde(default)]
    id: Option<String>,
    name: String,
    lat: f64,
    lng: f64,
}

#[derive(serde::Deserialize)]
pub(crate) struct RenameRequest {
    name: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/hubs", get(list_hubs))
        .route("/api/v3/hubs/{id}", patch(update_hub).delete(delete_hub))
        .route("/api/v3/hubs-create", post(create_hub))
        .route("/api/v3/organization", patch(update_organization))
}

async fn list_hubs(State(s): State<Arc<AppState>>, principal: AuthUser) -> ApiResult<Json<Value>> {
    require_staff(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let hubs = store(&s)?
        .hubs_for_org(&org)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        json!({ "data": hubs, "meta": { "organization": org } }),
    ))
}

async fn create_hub(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<HubRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let name = valid_name(&req.name)?;
    valid_coordinate(req.lat, req.lng)?;
    let id = req.id.unwrap_or_else(new_id);
    let created = store(&s)?
        .create_hub(&org, &id, name, req.lat, req.lng)
        .await
        .map_err(ApiError::Internal)?;
    if !created {
        return Err(ApiError::Conflict("hub could not be created".into()));
    }
    Ok(Json(json!({ "data": { "hubId": id } })))
}

async fn update_hub(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<HubRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let name = valid_name(&req.name)?;
    valid_coordinate(req.lat, req.lng)?;
    touched_or_404(
        store(&s)?
            .update_hub(&org, &id, name, req.lat, req.lng)
            .await
            .map_err(ApiError::Internal)?,
    )
}

async fn delete_hub(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    // Refuse rather than cascade: deleting a hub with tasks or teams attached
    // would orphan records the tenant still needs.
    if store(&s)?
        .hub_in_use(&org, &id)
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::Conflict(
            "hub still has tasks, teams, or audit records; reassign them first".into(),
        ));
    }
    // The pre-check is not atomic with the delete: a task/report written in
    // between trips a RESTRICT FK (SQLSTATE 23503) — answer 409, not 500.
    touched_or_404(
        store(&s)?
            .delete_hub(&org, &id)
            .await
            .map_err(|e| {
                if crate::store::pg::is_fk_violation(&e) {
                    ApiError::Conflict("hub is still referenced".into())
                } else {
                    ApiError::Internal(e)
                }
            })?,
    )
}

async fn update_organization(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<RenameRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let name = valid_name(&req.name)?;
    touched_or_404(
        store(&s)?
            .update_organization(&org, name)
            .await
            .map_err(ApiError::Internal)?,
    )
}
