use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, is_staff, org_of, require_staff, store};
use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::maps::{MAX_OPTIMIZE_WAYPOINTS, MapsClient, schematic_eta_seconds};
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
    let st = store(&s)?;
    let tasks = if is_staff(&principal) || principal.has_role(Role::Integration) {
        st.tasks_for_org(&org).await
    } else {
        st.tasks_for_driver(&org, &principal.subject).await
    }
    .map_err(ApiError::Internal)?;
    let hubs = st.hubs_for_org(&org).await.map_err(ApiError::Internal)?;
    let enriched = enrich_with_eta(tasks, &hubs);
    Ok(Json(json!({
        "data": enriched,
        "meta": { "mode": "live", "organization": org }
    })))
}

async fn get_task(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let org = org_of(&s, &principal.subject).await?;
    let st = store(&s)?;
    let task = st
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
    let hubs = st.hubs_for_org(&org).await.map_err(ApiError::Internal)?;
    task.map(|t| Json(json!({ "data": enrich_with_eta(vec![t], &hubs).into_iter().next().unwrap() })))
        .ok_or(ApiError::NotFound)
}

/// Inject `eta_minutes` into each task envelope, computed as a straight-line
/// ETA (30 km/h) from the task's hub to its first non-completed stop. Matches
/// the mobile's `schematicEtaMinutes` fallback so both sides agree when no
/// Google route has been planned. Zero when coordinates are missing.
fn enrich_with_eta(tasks: Vec<Value>, hubs: &[Value]) -> Vec<Value> {
    // Index hub coordinates by hub_id for O(1) lookup. Postgres gives `id`,
    // TypeDB gives `hub-id` — handle both.
    let hub_coords: std::collections::HashMap<&str, Coordinate> = hubs
        .iter()
        .filter_map(|h| {
            let hub = h.pointer("/hub")?;
            let id = hub
                .get("id")
                .or_else(|| hub.get("hub-id"))
                .or_else(|| hub.get("hub_id"))?
                .as_str()?;
            let lat = hub.get("lat")?.as_f64()?;
            let lng = hub.get("lng")?.as_f64()?;
            Some((id, Coordinate { lat, lng }))
        })
        .collect();

    tasks
        .into_iter()
        .map(|mut task| {
            let hub_id = task
                .pointer("/task/hub_id")
                .or_else(|| task.pointer("/task/hub-id"))
                .and_then(|v| v.as_str());
            let stops = task
                .get("stops")
                .and_then(|s| s.as_array())
                .cloned()
                .unwrap_or_default();
            // First stop that hasn't been completed/departed/skipped — the
            // next place the driver needs to be.
            let next_stop = stops.iter().find(|s| {
                let stage = s
                    .get("stage")
                    .or_else(|| s.get("status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending");
                !matches!(stage, "completed" | "departed" | "skipped")
            });
            let eta_minutes = match (hub_id, next_stop) {
                (Some(hid), Some(stop)) => {
                    let origin = hub_coords.get(hid);
                    let stop_lat = stop.get("lat").or_else(|| stop.get("latitude")).and_then(|v| v.as_f64());
                    let stop_lng = stop.get("lng").or_else(|| stop.get("longitude")).and_then(|v| v.as_f64());
                    match (origin, stop_lat, stop_lng) {
                        (Some(o), Some(lat), Some(lng)) => {
                            schematic_eta_seconds(*o, Coordinate { lat, lng }) / 60
                        }
                        _ => 0,
                    }
                }
                _ => 0,
            };
            // Inject into the task sub-object so the mobile parser's
            // `root['eta_minutes']` lookup finds it.
            if let Some(task_obj) = task.get_mut("task").and_then(|t| t.as_object_mut()) {
                task_obj.insert("eta_minutes".into(), json!(eta_minutes));
            }
            task
        })
        .collect()
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
    // Reject a malformed schedule here rather than letting the CHECK constraint
    // surface it as an opaque database error the caller cannot act on.
    task.validate_schedule().map_err(ApiError::BadRequest)?;
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
    task.validate_schedule().map_err(ApiError::BadRequest)?;
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
