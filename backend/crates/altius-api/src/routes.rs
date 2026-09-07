//! REST surface — `/api/v3` shape from Docs/ALTIUS_API_REFERENCE.md.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::agent;
use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::maps::{mode_marker, schematic_eta_seconds, MapsClient};
use crate::AppState;
use altius_core::{Coordinate, DeviceEvent, Role, Task};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/health", get(health))
        .route("/api/v3/auth/login", post(login))
        .route("/api/v3/tasks", get(list_tasks))
        .route("/api/v3/task/{id}", get(get_task))
        .route("/api/v3/task-create", post(create_task))
        .route("/api/v3/events", post(sync_events))
        .route("/api/v3/route/eta", post(eta))
        .route("/api/v3/route/optimize", post(optimize_route))
        .route("/api/v3/route/geocode", post(geocode))
        .route("/api/v3/agent/dispatch-suggestion", post(dispatch_suggestion))
        .route("/api/v3/agent/resume", post(agent_resume))
}

#[derive(serde::Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login(
    State(s): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<Value>> {
    if req.username.is_empty() || req.password.is_empty() {
        return Err(ApiError::BadRequest("username and password required".into()));
    }
    let params = [
        ("grant_type", "password"),
        ("client_id", s.config.keycloak.audience.as_str()),
        ("username", req.username.as_str()),
        ("password", req.password.as_str()),
    ];
    let resp = s
        .http
        .post(&s.config.keycloak.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| ApiError::Unavailable(format!("idp unreachable: {e}")))?;
    if resp.status().is_client_error() {
        return Err(ApiError::Unauthorized);
    }
    if !resp.status().is_success() {
        let status = resp.status();
        return Err(ApiError::Internal(anyhow::anyhow!("idp error {status}")));
    }
    let token: Value = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("idp token parse: {e}")))?;
    Ok(Json(json!({
        "accessToken": token.get("access_token").and_then(Value::as_str),
        "refreshToken": token.get("refresh_token").and_then(Value::as_str),
        "expiresIn": token.get("expires_in").and_then(Value::as_u64),
        "tokenType": token.get("token_type").and_then(Value::as_str),
    })))
}

async fn health(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "altius-api",
        "persistence": s.store.is_some(),
        "maps": s.config.google_maps_api_key.is_some(),
        "agent": s.config.openrouter_api_key.is_some(),
    }))
}

fn store(s: &Arc<AppState>) -> ApiResult<&crate::store::Store> {
    s.store
        .as_ref()
        .ok_or_else(|| ApiError::Unavailable("persistence not configured".into()))
}

/// Resolve the caller's org — TypeDB membership first, token claim as
/// fallback for tenants not yet synced.
async fn org_of(s: &Arc<AppState>, subject: &str, claim: Option<&String>) -> ApiResult<String> {
    match &s.store {
        Some(st) => Ok(st
            .organization_of(subject)
            .await
            .map_err(ApiError::Internal)?
            .or_else(|| claim.cloned())
            .ok_or(ApiError::Forbidden)?),
        None => claim.cloned().ok_or(ApiError::Forbidden),
    }
}

async fn list_tasks(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    let org = org_of(&s, &principal.subject, principal.organization_id.as_ref()).await?;
    let tasks = store(&s)?
        .tasks_for_org(&org)
        .await
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
    let org = org_of(&s, &principal.subject, principal.organization_id.as_ref()).await?;
    store(&s)?
        .task_by_id(&org, &id)
        .await
        .map_err(ApiError::Internal)?
        .map(|t| Json(json!({ "data": t })))
        .ok_or(ApiError::NotFound)
}

async fn create_task(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(task): Json<Task>,
) -> ApiResult<Json<Value>> {
    if principal.has_role(Role::Driver) {
        return Err(ApiError::Forbidden);
    }
    store(&s)?
        .create_task(&task)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": { "taskId": task.id } })))
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
        return Err(ApiError::BadRequest("batch must contain 1-500 events".into()));
    }
    let st = store(&s)?;
    let mut receipts = Vec::with_capacity(events.len());
    for ev in &events {
        receipts.push(st.record_event(ev).await.map_err(ApiError::Internal)?);
    }
    Ok(Json(json!({ "data": receipts })))
}

#[derive(serde::Deserialize)]
struct OptimizeRequest {
    origin: Coordinate,
    waypoints: Vec<Coordinate>,
}

async fn optimize_route(
    State(s): State<Arc<AppState>>,
    _principal: AuthUser,
    Json(req): Json<OptimizeRequest>,
) -> ApiResult<Json<Value>> {
    if req.waypoints.len() > crate::maps::MAX_OPTIMIZE_WAYPOINTS {
        return Err(ApiError::BadRequest(format!(
            "at most {} waypoints",
            crate::maps::MAX_OPTIMIZE_WAYPOINTS
        )));
    }
    let maps = MapsClient::new(s.http.clone(), s.config.google_maps_api_key.clone());
    let out = maps.optimize_stops(req.origin, &req.waypoints).await?;
    Ok(Json(json!({ "data": out })))
}

#[derive(serde::Deserialize)]
struct GeocodeRequest {
    address: String,
}

async fn geocode(
    State(s): State<Arc<AppState>>,
    _principal: AuthUser,
    Json(req): Json<GeocodeRequest>,
) -> ApiResult<Json<Value>> {
    let maps = MapsClient::new(s.http.clone(), s.config.google_maps_api_key.clone());
    let point = maps.geocode(&req.address).await?;
    Ok(Json(json!({ "data": { "location": point }, "meta": mode_marker(maps.enabled()) })))
}

fn dispatch_tools(state: &Arc<AppState>) -> Vec<agent::Tool> {
    vec![agent::Tool {
        name: "plan_route",
        description: "Order stops and return per-leg ETA in minutes",
        parameters: json!({
            "type": "object",
            "properties": {
                "stops": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["stops"],
        }),
        gated: true, // HITL: supervisor approves before dispatch writes anything
        exec: {
            let state = Arc::clone(state);
            Box::new(move |args| {
                let state = Arc::clone(&state);
                Box::pin(async move {
                    // Real solver input: pairwise leg durations via Distance Matrix.
                    let points: Vec<Coordinate> = args
                        .get("stops")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(|s| serde_json::from_value::<Coordinate>(s.clone()).ok())
                                .collect()
                        })
                        .unwrap_or_default();
                    if points.len() < 2 {
                        return agent::ToolOutput::Executed(json!({
                            "error": "plan_route needs >=2 coordinate stops"
                        }));
                    }
                    let maps = MapsClient::new(
                        state.http.clone(),
                        state.config.google_maps_api_key.clone(),
                    );
                    let matrix = if maps.enabled() {
                        match maps.distance_matrix_seconds(&points, &points).await {
                            Ok(m) => m,
                            Err(e) => {
                                return agent::ToolOutput::Executed(json!({
                                    "error": format!("distance matrix: {e}")
                                }))
                            }
                        }
                    } else {
                        points
                            .iter()
                            .map(|a| {
                                points
                                    .iter()
                                    .map(|b| Some(schematic_eta_seconds(*a, *b)))
                                    .collect()
                            })
                            .collect()
                    };
                    agent::ToolOutput::Executed(json!({
                        "durations_seconds": matrix,
                        "source": if maps.enabled() { "live" } else { "demo" },
                    }))
                })
            })
        },
    }]
}

#[derive(serde::Deserialize)]
struct ResumeRequest {
    state: Value,
    call: agent::ToolCall,
    decision: String,
    #[serde(default)]
    reason: Option<String>,
}

async fn agent_resume(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<ResumeRequest>,
) -> ApiResult<Json<Value>> {
    if !(principal.has_role(Role::Admin) || principal.has_role(Role::Supervisor)) {
        return Err(ApiError::Forbidden);
    }
    let key = s
        .config
        .openrouter_api_key
        .clone()
        .ok_or_else(|| ApiError::Unavailable("OPENROUTER_API_KEY not configured".into()))?;
    let decision = match req.decision.as_str() {
        "approve" => agent::Decision::Approve,
        "reject" => agent::Decision::Reject {
            reason: req.reason.unwrap_or_else(|| "rejected".into()),
        },
        _ => return Err(ApiError::BadRequest("decision must be approve|reject".into())),
    };
    match agent::resume(
        &s.http,
        &key,
        &s.config.openrouter_model,
        &dispatch_tools(&s),
        req.state,
        &req.call,
        decision,
    )
    .await?
    {
        agent::AgentRun::Finished(text) => Ok(Json(json!({ "data": { "text": text } }))),
        agent::AgentRun::AwaitingApproval { call, messages_json } => Ok(Json(json!({
            "data": { "status": "awaiting_approval", "call": call },
            "meta": { "state": messages_json }
        }))),
    }
}

#[derive(serde::Deserialize)]
struct EtaRequest {
    from: Coordinate,
    to: Coordinate,
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
    Ok(Json(json!({ "data": { "eta_seconds": seconds }, "meta": marker })))
}

async fn dispatch_suggestion(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(input): Json<Value>,
) -> ApiResult<Json<Value>> {
    if !(principal.has_role(Role::Admin) || principal.has_role(Role::Supervisor)) {
        return Err(ApiError::Forbidden);
    }
    let key = s
        .config
        .openrouter_api_key
        .clone()
        .ok_or_else(|| ApiError::Unavailable("OPENROUTER_API_KEY not configured".into()))?;

    let tools = dispatch_tools(&s);

    match agent::run(
        &s.http,
        &key,
        &s.config.openrouter_model,
        "You are Altius dispatch. Suggest an efficient stop order and flag risks. Never write data without approval.",
        &serde_json::to_string(&input).unwrap_or_default(),
        &tools,
    )
    .await?
    {
        agent::AgentRun::Finished(text) => Ok(Json(json!({ "data": { "text": text } }))),
        agent::AgentRun::AwaitingApproval { call, messages_json } => Ok(Json(json!({
            "data": { "status": "awaiting_approval", "call": call },
            "meta": { "state": messages_json }
        }))),
    }
}
