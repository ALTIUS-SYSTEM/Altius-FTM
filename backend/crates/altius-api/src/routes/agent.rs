use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use super::AuthUser;
use crate::AppState;
use crate::agent::{AgentRun, Decision, Tool, ToolOutput, resume, run};
use crate::error::{ApiError, ApiResult};
use crate::maps::{MapsClient, schematic_eta_seconds};
use altius_core::{Coordinate, Role};

#[derive(serde::Deserialize)]
pub(crate) struct ResumeRequest {
    state: String,
    decision: String,
    #[serde(default)]
    reason: Option<String>,
}

/// Google Distance Matrix bills origins x destinations and caps a request at
/// 100 elements. `plan_route` sends the stop list against itself, so N stops
/// cost N^2 — the model chooses N, and nothing else bounds it.
pub(crate) const MAX_PLAN_ROUTE_STOPS: usize = 10;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v3/agent/dispatch-suggestion",
            post(dispatch_suggestion),
        )
        .route("/api/v3/agent/resume", post(agent_resume))
}

/// Parse the `stops` argument into coordinates.
///
/// Rejects rather than silently dropping. The previous version `filter_map`ped
/// unparseable entries away, so a model that followed the *declared* schema —
/// which said `items: {type: string}` while this code deserialized `{lat,lng}`
/// objects — produced an empty list and the tool answered "needs >=2 stops"
/// every time. A tool whose schema and parser disagree never works; it only
/// looks like the model is being unhelpful.
pub(crate) fn parse_plan_route_stops(args: &Value) -> Result<Vec<Coordinate>, String> {
    let Some(items) = args.get("stops").and_then(Value::as_array) else {
        return Err("stops must be an array of {lat, lng} objects".into());
    };
    if items.len() < 2 {
        return Err("plan_route needs at least 2 stops".into());
    }
    if items.len() > MAX_PLAN_ROUTE_STOPS {
        return Err(format!(
            "plan_route accepts at most {MAX_PLAN_ROUTE_STOPS} stops per call"
        ));
    }
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let c: Coordinate = serde_json::from_value(item.clone())
            .map_err(|_| format!("stop {i} is not a {{lat, lng}} object"))?;
        if !c.lat.is_finite()
            || !c.lng.is_finite()
            || !(-90.0..=90.0).contains(&c.lat)
            || !(-180.0..=180.0).contains(&c.lng)
        {
            return Err(format!("stop {i} has coordinates outside the valid range"));
        }
        out.push(c);
    }
    Ok(out)
}

/// Tools the dispatch agent may call.
///
/// Built per request with the caller's principal in scope. A tool that reads or
/// writes org data must scope itself by `principal` — the model's arguments are
/// untrusted input steered by whatever text reached the prompt, and the closure
/// otherwise runs with the backend's ambient credentials.
fn dispatch_tools(state: &Arc<AppState>, _principal: &AuthUser) -> Vec<Tool> {
    vec![Tool {
        name: "plan_route".into(),
        description: "Order stops and return pairwise leg durations in seconds. Stops are \
             {lat, lng} objects."
            .into(),
        // The schema the model is given must match what the executor accepts,
        // or every compliant call fails.
        parameters: json!({
            "type": "object",
            "properties": {
                "stops": {
                    "type": "array",
                    "minItems": 2,
                    "maxItems": MAX_PLAN_ROUTE_STOPS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "lat": { "type": "number", "minimum": -90, "maximum": 90 },
                            "lng": { "type": "number", "minimum": -180, "maximum": 180 }
                        },
                        "required": ["lat", "lng"]
                    }
                }
            },
            "required": ["stops"],
        }),
        gated: true, // HITL: supervisor approves before dispatch writes anything
        exec: {
            let state = Arc::clone(state);
            Box::new(move |args| {
                let state = Arc::clone(&state);
                Box::pin(async move {
                    // Errors go back to the model as a tool result so it can
                    // correct itself, rather than aborting the run.
                    let points = match parse_plan_route_stops(&args) {
                        Ok(p) => p,
                        Err(e) => return ToolOutput::Executed(json!({ "error": e })),
                    };
                    let maps = MapsClient::new(
                        state.http.clone(),
                        state.config.google_maps_api_key.clone(),
                    );
                    let matrix = if maps.enabled() {
                        match maps.distance_matrix_seconds(&points, &points).await {
                            Ok(m) => m,
                            Err(e) => {
                                return ToolOutput::Executed(json!({
                                    "error": format!("distance matrix: {e}")
                                }));
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
                    ToolOutput::Executed(json!({
                        "durations_seconds": matrix,
                        "source": if maps.enabled() { "live" } else { "demo" },
                    }))
                })
            })
        },
    }]
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
        "approve" => Decision::Approve,
        "reject" => Decision::Reject {
            reason: req.reason.unwrap_or_else(|| "rejected".into()),
        },
        _ => {
            return Err(ApiError::BadRequest(
                "decision must be approve|reject".into(),
            ));
        }
    };
    let secret = s
        .config
        .agent_state_secret
        .as_deref()
        .ok_or_else(|| ApiError::Unavailable("AGENT_STATE_SECRET not configured".into()))?;
    match resume(
        &s.http,
        &key,
        &s.config.openrouter_model,
        &dispatch_tools(&s, &principal),
        &req.state,
        secret,
        &principal.subject,
        decision,
    )
    .await?
    {
        AgentRun::Finished(text) => Ok(Json(json!({ "data": { "text": text } }))),
        AgentRun::AwaitingApproval { call, state } => Ok(Json(json!({
            "data": { "status": "awaiting_approval", "call": call },
            "meta": { "state": state }
        }))),
    }
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

    let tools = dispatch_tools(&s, &principal);

    let secret = s
        .config
        .agent_state_secret
        .as_deref()
        .ok_or_else(|| ApiError::Unavailable("AGENT_STATE_SECRET not configured".into()))?;
    match run(
        &s.http,
        &key,
        &s.config.openrouter_model,
        "You are Altius dispatch. Suggest an efficient stop order and flag risks. Never write data without approval.",
        &serde_json::to_string(&input).unwrap_or_default(),
        &tools,
        secret,
        &principal.subject,
    )
    .await?
    {
        AgentRun::Finished(text) => Ok(Json(json!({ "data": { "text": text } }))),
        AgentRun::AwaitingApproval { call, state } => Ok(Json(json!({
            "data": { "status": "awaiting_approval", "call": call },
            "meta": { "state": state }
        }))),
    }
}
