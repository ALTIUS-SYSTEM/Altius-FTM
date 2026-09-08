//! OpenRouter agent loop — Rust port of the `go-agent` tool-call semantics:
//! model → tool calls → tool results → repeat, with a step cap and an HITL
//! gate that pauses execution for human approval.

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::ApiError;

const OPENROUTER_CHAT: &str = "https://openrouter.ai/api/v1/chat/completions";
const MAX_STEPS: usize = 8;
/// Cap on tool calls honoured per model turn. The model's `tool_calls` array is
/// third-party output steerable by any text that reaches the prompt.
const MAX_TOOL_CALLS_PER_STEP: usize = 4;
/// A paused run must be resumed promptly; a sealed state is not a session.
const STATE_TTL_SECS: u64 = 900;

/// A paused run, sealed as an HS256 JWT.
///
/// The conversation and the pending tool call are carried *inside* the seal,
/// bound to the approving subject and to a remaining-step budget. `resume`
/// executes the sealed call, never one supplied in the request body — otherwise
/// "approve what the model proposed" and "run any tool with any arguments,
/// having invented the whole transcript" are the same request.
#[derive(Debug, Serialize, Deserialize)]
struct SealedState {
    sub: String,
    exp: u64,
    messages: Vec<Message>,
    call: ToolCall,
    steps_left: usize,
}

fn seal(
    secret: &str,
    subject: &str,
    messages: &[Message],
    call: &ToolCall,
    steps_left: usize,
) -> Result<String, ApiError> {
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() + STATE_TTL_SECS)
        .unwrap_or(0);
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &SealedState {
            sub: subject.to_string(),
            exp,
            messages: messages.to_vec(),
            call: call.clone(),
            steps_left,
        },
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("seal agent state: {e}")))
}

fn unseal(secret: &str, subject: &str, token: &str) -> Result<SealedState, ApiError> {
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_required_spec_claims(&["exp", "sub"]);
    validation.validate_exp = true;
    let data = jsonwebtoken::decode::<SealedState>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| ApiError::BadRequest("agent state is invalid or expired".into()))?;
    // An approval is personal: the resuming caller must be the one the run was
    // handed to, not merely someone else holding a supervisor token.
    if data.claims.sub != subject {
        return Err(ApiError::Forbidden);
    }
    Ok(data.claims)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ToolOutput {
    /// Auto-executed result fed back into the loop.
    Executed(Value),
    /// Tool refuses to run without human approval.
    AwaitingApproval,
}

// `Sync` is required: axum handlers hold `Vec<Tool>` across an await, so the
// handler future must be Send + Sync or the Handler bound silently fails.
pub type ToolFn =
    Box<dyn Fn(Value) -> Pin<Box<dyn Future<Output = ToolOutput> + Send>> + Send + Sync>;

pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub gated: bool,
    pub exec: ToolFn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<RawToolCall>>,
    tool_call_id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawToolCall {
    id: String,
    #[serde(rename = "function")]
    function: RawFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawFunction {
    name: String,
    arguments: String,
}

/// Run the agent loop. Returns the final assistant text, or pauses with the
/// gated call awaiting approval (the caller persists state and resumes).
#[allow(clippy::too_many_arguments)]
pub async fn run(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    system: &str,
    input: &str,
    tools: &[Tool],
    secret: &str,
    subject: &str,
) -> Result<AgentRun, ApiError> {
    let mut messages = vec![
        Message {
            role: "system".into(),
            content: Some(system.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
        Message {
            role: "user".into(),
            content: Some(input.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
    ];
    drive(http, api_key, model, tools, &mut messages, secret, subject, MAX_STEPS).await
}

/// Resume a paused run: `approved` feeds the gated call's result back into
/// the loop, `rejected` appends a denial tool-result and lets the model
/// continue without executing the tool.
#[allow(clippy::too_many_arguments)]
pub async fn resume(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    tools: &[Tool],
    state_token: &str,
    secret: &str,
    subject: &str,
    decision: Decision,
) -> Result<AgentRun, ApiError> {
    // Everything the run needs comes out of the seal: the transcript, the tool
    // call the model actually proposed, and the remaining step budget. The
    // request body contributes only the approve/reject decision.
    let sealed = unseal(secret, subject, state_token)?;
    let mut messages = sealed.messages;
    let pending_call = sealed.call;

    let content = match decision {
        Decision::Approve => {
            let tool = tools
                .iter()
                .find(|t| t.name == pending_call.name)
                .ok_or_else(|| {
                    ApiError::BadRequest(format!("unknown tool {}", pending_call.name))
                })?;
            // Only a gated tool can legitimately sit in a seal awaiting
            // approval; anything else means the seal was built wrong.
            if !tool.gated {
                return Err(ApiError::BadRequest(
                    "pending call is not an approval-gated tool".into(),
                ));
            }
            validate_args(tool, &pending_call.arguments)?;
            match (tool.exec)(pending_call.arguments.clone()).await {
                ToolOutput::Executed(v) => v.to_string(),
                ToolOutput::AwaitingApproval => {
                    let state =
                        seal(secret, subject, &messages, &pending_call, sealed.steps_left)?;
                    return Ok(AgentRun::AwaitingApproval {
                        call: pending_call,
                        state,
                    });
                }
            }
        }
        Decision::Reject { reason } => json!({
            "error": "tool call rejected",
            "reason": reason,
        })
        .to_string(),
    };

    messages.push(Message {
        role: "tool".into(),
        content: Some(content),
        tool_calls: None,
        tool_call_id: Some(pending_call.id.clone()),
        name: Some(pending_call.name.clone()),
    });

    drive(
        http,
        api_key,
        model,
        tools,
        &mut messages,
        secret,
        subject,
        sealed.steps_left,
    )
    .await
}

/// Shape-check tool arguments against the tool's advertised JSON Schema before
/// dispatch. The schema is sent to the model but was never enforced on the way
/// back, so `exec` received whatever the model emitted.
fn validate_args(tool: &Tool, args: &Value) -> Result<(), ApiError> {
    if !args.is_object() {
        return Err(ApiError::BadRequest(format!(
            "tool {} expects an object argument",
            tool.name
        )));
    }
    let Some(required) = tool.parameters.get("required").and_then(Value::as_array) else {
        return Ok(());
    };
    for key in required.iter().filter_map(Value::as_str) {
        if args.get(key).is_none() {
            return Err(ApiError::BadRequest(format!(
                "tool {} is missing required argument {key}",
                tool.name
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub enum Decision {
    Approve,
    Reject { reason: String },
}

/// The tool-call loop: model → tool calls → results → repeat.
#[allow(clippy::too_many_arguments)]
async fn drive(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    tools: &[Tool],
    messages: &mut Vec<Message>,
    secret: &str,
    subject: &str,
    steps_left: usize,
) -> Result<AgentRun, ApiError> {

    let tool_defs: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }
            })
        })
        .collect();

    for step in 0..steps_left.min(MAX_STEPS) {
        let remaining = steps_left.min(MAX_STEPS) - step - 1;
        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tool_defs,
        });
        let res: ChatResponse = http
            .post(OPENROUTER_CHAT)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::upstream("openrouter", e))?
            .json()
            .await
            .map_err(|e| ApiError::upstream("openrouter", e))?;

        let msg = res
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or_else(|| ApiError::Unavailable("empty choices".into()))?;

        let calls = msg.tool_calls.clone().unwrap_or_default();
        messages.push(msg);

        if calls.is_empty() {
            let text = messages
                .last()
                .and_then(|m| m.content.clone())
                .unwrap_or_default();
            return Ok(AgentRun::Finished(text));
        }

        let _over_budget = calls.len() > MAX_TOOL_CALLS_PER_STEP;
        for call in calls.into_iter().take(MAX_TOOL_CALLS_PER_STEP) {
            let tool = tools.iter().find(|t| t.name == call.function.name);
            let Some(tool) = tool else {
                messages.push(Message {
                    role: "tool".into(),
                    content: Some(json!({"error": "unknown tool"}).to_string()),
                    tool_calls: None,
                    tool_call_id: Some(call.id.clone()),
                    name: Some(call.function.name.clone()),
                });
                continue;
            };

            if tool.gated {
                let args: Value =
                    serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));
                let pending = ToolCall {
                    id: call.id,
                    name: tool.name.to_string(),
                    arguments: args,
                };
                let state = seal(secret, subject, messages, &pending, remaining)?;
                return Ok(AgentRun::AwaitingApproval { call: pending, state });
            }

            let args: Value =
                serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));
            let output = (tool.exec)(args.clone()).await;
            let content = match output {
                ToolOutput::Executed(v) => v.to_string(),
                ToolOutput::AwaitingApproval => {
                    let pending = ToolCall {
                        id: call.id,
                        name: tool.name.to_string(),
                        arguments: args,
                    };
                    let state = seal(secret, subject, messages, &pending, remaining)?;
                    return Ok(AgentRun::AwaitingApproval { call: pending, state });
                }
            };
            messages.push(Message {
                role: "tool".into(),
                content: Some(content),
                tool_calls: None,
                tool_call_id: Some(call.id),
                name: Some(tool.name.to_string()),
            });
        }
    }

    let text = messages
        .last()
        .and_then(|m| m.content.clone())
        .unwrap_or_default();
    Ok(AgentRun::Finished(text))
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Clone)]
pub enum AgentRun {
    Finished(String),
    AwaitingApproval { call: ToolCall, state: String },
}
