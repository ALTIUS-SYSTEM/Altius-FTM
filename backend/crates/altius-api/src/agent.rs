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
    /// Human approval required; the loop pauses with this call pending.
    AwaitingApproval,
}

type Exec =
    Box<dyn Fn(Value) -> Pin<Box<dyn Future<Output = ToolOutput> + Send>> + Send + Sync>;

pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
    /// Whether a human must approve before `exec` runs (HITL gate).
    pub gated: bool,
    pub exec: Exec,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<RawToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RawToolCall {
    id: String,
    function: RawFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RawFunction {
    name: String,
    arguments: String,
}

/// Run the agent loop. Returns the final assistant text, or pauses with the
/// gated call awaiting approval (the caller persists state and resumes).
pub async fn run(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    system: &str,
    input: &str,
    tools: &[Tool],
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
    drive(http, api_key, model, tools, &mut messages).await
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
    messages_json: Value,
    pending_call: &ToolCall,
    decision: Decision,
) -> Result<AgentRun, ApiError> {
    let mut messages: Vec<Message> = serde_json::from_value(messages_json)
        .map_err(|e| ApiError::BadRequest(format!("invalid saved state: {e}")))?;

    let content = match decision {
        Decision::Approve => {
            let tool = tools
                .iter()
                .find(|t| t.name == pending_call.name)
                .ok_or_else(|| {
                    ApiError::BadRequest(format!("unknown tool {}", pending_call.name))
                })?;
            match (tool.exec)(pending_call.arguments.clone()).await {
                ToolOutput::Executed(v) => v.to_string(),
                ToolOutput::AwaitingApproval => {
                    return Ok(AgentRun::AwaitingApproval {
                        call: pending_call.clone(),
                        messages_json: serde_json::to_value(&messages)
                            .unwrap_or(json!([])),
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

    drive(http, api_key, model, tools, &mut messages).await
}

#[derive(Debug, Clone)]
pub enum Decision {
    Approve,
    Reject { reason: String },
}

/// The tool-call loop: model → tool calls → results → repeat.
async fn drive(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    tools: &[Tool],
    messages: &mut Vec<Message>,
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

    for _step in 0..MAX_STEPS {
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

        for call in calls {
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
                return Ok(AgentRun::AwaitingApproval {
                    call: ToolCall {
                        id: call.id,
                        name: tool.name.to_string(),
                        arguments: args,
                    },
                    messages_json: serde_json::to_value(&messages).unwrap_or(json!([])),
                });
            }

            let args: Value =
                serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));
            let output = (tool.exec)(args.clone()).await;
            let content = match output {
                ToolOutput::Executed(v) => v.to_string(),
                ToolOutput::AwaitingApproval => {
                    return Ok(AgentRun::AwaitingApproval {
                        call: ToolCall {
                            id: call.id,
                            name: tool.name.to_string(),
                            arguments: args,
                        },
                        messages_json: serde_json::to_value(&messages)
                            .unwrap_or(json!([])),
                    });
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

    Err(ApiError::Unavailable(format!(
        "agent exceeded {MAX_STEPS} steps"
    )))
}

pub enum AgentRun {
    Finished(String),
    AwaitingApproval {
        call: ToolCall,
        messages_json: Value,
    },
}
