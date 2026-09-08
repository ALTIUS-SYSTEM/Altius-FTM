use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, is_staff, org_of, require_staff, store};
use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::maps::{MAX_OPTIMIZE_WAYPOINTS, MapsClient};
use altius_core::{Coordinate, DeviceEvent, Role, StopAction, Task};

#[derive(serde::Deserialize)]
pub(crate) struct OptimizeRequest {
    origin: Coordinate,
    waypoints: Vec<Coordinate>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/tasks", get(list_tasks))
        .route(
            "/api/v3/task/{id}",
            get(get_task).put(update_task).delete(delete_task),
        )
        .route("/api/v3/task-create", post(create_task))
        .route("/api/v3/events", post(sync_events))
        .route("/api/v3/route/optimize", post(optimize_route))
}

async fn list_tasks(State(s): State<Arc<AppState>>, principal: AuthUser) -> ApiResult<Json<Value>> {
    let org = org_of(&s, &principal.subject).await?;
    // A driver pulls only their assignments — the mobile board has no claim
    // flow, so an org-wide list just leaks colleagues' work. Staff and
    // integration service accounts read the full board.
    let tasks = if is_staff(&principal) || principal.has_role(Role::Integration) {
        store(&s)?.tasks_for_org(&org).await
    } else {
        store(&s)?.tasks_for_driver(&org, &principal.subject).await
    }
    .map_err(ApiError::Internal)?;
    Ok(Json(json!({
        "data": tasks,
        "meta": { "mode": "live", "organization": org }
    })))
}

async fn get_task(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let org = org_of(&s, &principal.subject).await?;
    let task = store(&s)?
        .task_by_id(&org, &id)
        .await
        .map_err(ApiError::Internal)?;
    // Drivers may fetch only their own assignments — same rule as list_tasks.
    if !is_staff(&principal) && !principal.has_role(Role::Integration) {
        let assignee = task
            .as_ref()
            .and_then(|t| t.pointer("/task/assignee"))
            .and_then(|a| a.as_str());
        if assignee != Some(principal.subject.as_str()) {
            return Err(ApiError::NotFound);
        }
    }
    task.map(|t| Json(json!({ "data": t })))
        .ok_or(ApiError::NotFound)
}

async fn create_task(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(task): Json<Task>,
) -> ApiResult<Json<Value>> {
    // Allow-list, not deny-list: a deny on Driver alone let every token that
    // carried no recognised role through.
    require_staff(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    // Never trust body tenant_id for scoping — org comes from the JWT link.
    let mut task = task;
    task.tenant_id = org.clone();
    store(&s)?
        .create_task(&org, &task)
        .await
        // The store rejects a hub outside the caller's org; that is the
        // client's mistake, not an internal fault.
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(json!({ "data": { "taskId": task.id } })))
}

async fn update_task(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
    Json(mut task): Json<Task>,
) -> ApiResult<Json<Value>> {
    require_staff(&principal)?;
    if task.id != id {
        return Err(ApiError::BadRequest(
            "path id and body id must match".into(),
        ));
    }
    let org = org_of(&s, &principal.subject).await?;
    // Never trust body tenant_id for scoping — org comes from the JWT link.
    task.tenant_id = org.clone();
    store(&s)?.update_task(&org, &task).await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            ApiError::NotFound
        } else {
            ApiError::BadRequest(msg)
        }
    })?;
    Ok(Json(json!({ "data": { "taskId": task.id } })))
}

async fn delete_task(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_staff(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    match store(&s)?
        .delete_task(&org, &id)
        .await
        .map_err(ApiError::Internal)?
    {
        None => Err(ApiError::NotFound),
        Some(false) => Err(ApiError::Conflict(
            "task is in progress; cancel it before deleting".into(),
        )),
        Some(true) => Ok(Json(json!({ "data": { "deleted": id } }))),
    }
}

async fn sync_events(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(events): Json<Vec<DeviceEvent>>,
) -> ApiResult<Json<Value>> {
    if !principal.has_role(Role::Driver) {
        return Err(ApiError::Forbidden);
    }
    if events.is_empty() || events.len() > 500 {
        return Err(ApiError::BadRequest(
            "batch must contain 1-500 events".into(),
        ));
    }
    let scope = store(&s)?
        .organization_and_hub_of(&principal.subject)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Forbidden)?;
    for ev in &events {
        if ev.tenant_id != scope.0 || ev.hub_id != scope.1 {
            return Err(ApiError::BadRequest(
                "event tenant/hub scope mismatch".into(),
            ));
        }
        // Mirror DeviceEventSchema.superRefine: skip requires a non-empty reason
        // before we hit the DB CHECK (device_events_skip_requires_reason).
        if matches!(ev.action, StopAction::Skip)
            && ev.reason.as_ref().is_none_or(|r| r.trim().is_empty())
        {
            return Err(ApiError::BadRequest(
                "skip action requires a non-empty reason".into(),
            ));
        }
    }
    let st = store(&s)?;
    let mut receipts = Vec::with_capacity(events.len());
    for ev in &events {
        receipts.push(
            st.record_event(&scope.0, &principal.subject, ev)
                .await
                .map_err(ApiError::Internal)?,
        );
    }
    Ok(Json(json!({ "data": receipts })))
}

async fn optimize_route(
    State(s): State<Arc<AppState>>,
    _principal: AuthUser,
    Json(req): Json<OptimizeRequest>,
) -> ApiResult<Json<Value>> {
    if req.waypoints.len() > MAX_OPTIMIZE_WAYPOINTS {
        return Err(ApiError::BadRequest(format!(
            "at most {} waypoints",
            MAX_OPTIMIZE_WAYPOINTS
        )));
    }
    let maps = MapsClient::new(s.http.clone(), s.config.google_maps_api_key.clone());
    let out = maps.optimize_stops(req.origin, &req.waypoints).await?;
    Ok(Json(json!({ "data": out })))
}
