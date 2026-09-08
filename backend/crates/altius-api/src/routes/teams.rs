use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, org_of, require_admin, require_staff, store, touched_or_404, valid_name};
use crate::AppState;
use crate::error::{ApiError, ApiResult};
use altius_core::new_id;

#[derive(serde::Deserialize)]
pub(crate) struct TeamRequest {
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    shift: String,
    #[serde(default)]
    hub_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct MemberRequest {
    subject: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/teams", get(list_teams).post(create_team))
        .route("/api/v3/teams/{id}", patch(update_team).delete(delete_team))
        .route(
            "/api/v3/teams/{id}/members",
            post(add_team_member).delete(remove_team_member),
        )
}

async fn list_teams(State(s): State<Arc<AppState>>, principal: AuthUser) -> ApiResult<Json<Value>> {
    require_staff(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let teams = store(&s)?
        .teams_for_org(&org)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": teams })))
}

async fn create_team(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<TeamRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let (org, own_hub) = store(&s)?
        .organization_and_hub_of(&principal.subject)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Forbidden)?;
    let name = valid_name(&req.name)?;
    let id = req.id.unwrap_or_else(new_id);
    let hub = req.hub_id.unwrap_or(own_hub);
    let created = store(&s)?
        .create_team(&org, &hub, &id, name, req.shift.trim())
        .await
        .map_err(ApiError::Internal)?;
    if !created {
        return Err(ApiError::Conflict(
            "team could not be created; check the hub belongs to your organization".into(),
        ));
    }
    Ok(Json(json!({ "data": { "teamId": id } })))
}

async fn update_team(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<TeamRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let name = valid_name(&req.name)?;
    touched_or_404(
        store(&s)?
            .update_team(&org, &id, name, req.shift.trim())
            .await
            .map_err(ApiError::Internal)?,
    )
}

async fn delete_team(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    touched_or_404(
        store(&s)?
            .delete_team(&org, &id)
            .await
            .map_err(ApiError::Internal)?,
    )
}

async fn add_team_member(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<MemberRequest>,
) -> ApiResult<Json<Value>> {
    require_staff(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    touched_or_404(
        store(&s)?
            .add_team_member(&org, &id, req.subject.trim())
            .await
            .map_err(ApiError::Internal)?,
    )
}

async fn remove_team_member(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<MemberRequest>,
) -> ApiResult<Json<Value>> {
    require_staff(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    touched_or_404(
        store(&s)?
            .remove_team_member(&org, &id, req.subject.trim())
            .await
            .map_err(ApiError::Internal)?,
    )
}
