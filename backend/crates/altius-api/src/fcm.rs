//! FCM v1 HTTP batch send with Google Application Default Credentials.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;

use crate::error::{ApiError, ApiResult};

const FCM_BATCH_URL: &str = "https://fcm.googleapis.com/batch";
const FCM_SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";

fn project_id_from_credentials(path: Option<&str>) -> Option<String> {
    let path = path?;
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("project_id")
                .and_then(|p| p.as_str())
                .map(String::from)
        })
}

/// Result for a single token in a batch.
#[derive(Debug, Clone)]
pub struct FcmDelivery {
    pub token: String,
    pub name: Option<String>,
    pub error: Option<String>,
}

/// FCM v1 client using Google ADC.
#[derive(Clone)]
pub struct FcmClient {
    http: reqwest::Client,
    project_id: String,
    token_provider: Arc<dyn gcp_auth::TokenProvider>,
}

impl FcmClient {
    pub async fn from_config(cfg: &crate::config::Config) -> ApiResult<Self> {
        let project_id = cfg
            .fcm_project_id
            .clone()
            .or_else(|| project_id_from_credentials(cfg.fcm_credentials_path.as_deref()))
            .ok_or_else(|| ApiError::Unavailable("fcm project id not configured".into()))?;
        Self::new(project_id).await
    }

    pub async fn new(project_id: String) -> ApiResult<Self> {
        let token_provider = gcp_auth::provider()
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("fcm adc provider: {e}")))?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ApiError::Internal(e.into()))?;
        Ok(Self {
            http,
            project_id,
            token_provider,
        })
    }

    /// Send a notification to many tokens in one FCM v1 batch request.
    pub async fn send_batch(
        &self,
        tokens: &[String],
        title: &str,
        body: &str,
    ) -> ApiResult<Vec<FcmDelivery>> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let token = self
            .token_provider
            .token(&[FCM_SCOPE])
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("fcm token: {e}")))?;
        let boundary = uuid::Uuid::new_v4().to_string();
        let mut payload = Vec::new();
        for t in tokens {
            let part = batch_part(&self.project_id, t, title, body)?;
            payload.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            payload.extend_from_slice(b"Content-Type: application/http\r\n");
            payload.extend_from_slice(b"Content-Transfer-Encoding: binary\r\n\r\n");
            payload.extend_from_slice(part.as_bytes());
            payload.extend_from_slice(b"\r\n");
        }
        payload.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let res = self
            .http
            .post(FCM_BATCH_URL)
            .header(
                "Content-Type",
                format!("multipart/mixed; boundary={boundary}"),
            )
            .bearer_auth(token.as_str())
            .body(payload)
            .send()
            .await
            .map_err(|e| ApiError::upstream("fcm", e))?;
        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %text, "fcm batch request failed");
            return Err(ApiError::Unavailable("fcm send failed".into()));
        }
        let ct = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = res
            .bytes()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
        parse_batch_response(tokens, &ct, &body)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("fcm batch parse: {e}")))
    }
}

fn batch_part(project_id: &str, token: &str, title: &str, body: &str) -> ApiResult<String> {
    let msg = serde_json::json!({
        "message": {
            "token": token,
            "notification": { "title": title, "body": body }
        }
    });
    let body_json = serde_json::to_string(&msg).map_err(|e| ApiError::Internal(e.into()))?;
    Ok(format!(
        "POST /v1/projects/{project_id}/messages:send HTTP/1.1\r\n\
         Content-Type: application/json; charset=utf-8\r\n\
         Accept: application/json\r\n\
         Content-Length: {len}\r\n\r\n\
         {body_json}",
        len = body_json.len()
    ))
}

/// Parse the multipart/mixed response returned by the FCM batch endpoint.
fn parse_batch_response(
    tokens: &[String],
    content_type: &str,
    body: &[u8],
) -> anyhow::Result<Vec<FcmDelivery>> {
    let boundary = content_type
        .split(';')
        .map(str::trim)
        .find_map(|p| p.strip_prefix("boundary="))
        .context("missing multipart boundary")?;
    let text = std::str::from_utf8(body).context("fcm response is not utf-8")?;
    let mut results = Vec::new();
    let mut token_iter = tokens.iter();
    for raw in text.split(&format!("--{boundary}")) {
        let raw = raw.trim();
        if raw.is_empty() || raw == "--" {
            continue;
        }
        let Some(inner) = raw
            .find("\r\n\r\n")
            .map(|i| &raw[i + 4..])
            .or_else(|| raw.find("\n\n").map(|i| &raw[i + 2..]))
        else {
            continue;
        };
        let Some(token) = token_iter.next() else {
            break;
        };
        let name = parse_success_name(inner);
        let error = if name.is_none() {
            Some(extract_error(inner))
        } else {
            None
        };
        results.push(FcmDelivery {
            token: token.clone(),
            name,
            error,
        });
    }
    // If parsing produced fewer results than tokens, append unknowns so we never
    // drop a token silently.
    for token in token_iter {
        results.push(FcmDelivery {
            token: token.clone(),
            name: None,
            error: Some("no response part".into()),
        });
    }
    Ok(results)
}

/// Byte offset just past the header/body separator of a multipart part.
/// `+4` for CRLFCRLF, `+2` for the bare-LF fallback — adding 4 unconditionally
/// sliced two bytes into the JSON body and broke parsing on the `\n\n` path.
fn body_start(inner: &str) -> Option<usize> {
    inner
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .or_else(|| inner.find("\n\n").map(|i| i + 2))
}

fn parse_success_name(inner: &str) -> Option<String> {
    let first = inner.lines().next().unwrap_or("");
    if !first.contains("200") {
        return None;
    }
    let json = &inner[body_start(inner)?..];
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
}

fn extract_error(inner: &str) -> String {
    if let Some(json_start) = body_start(inner) {
        let json = &inner[json_start..];
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(msg) = v
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
            {
                return msg.to_string();
            }
            if let Some(code) = v
                .get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_i64())
            {
                return format!("fcm error {code}");
            }
        }
    }
    inner
        .lines()
        .next()
        .unwrap_or("unknown fcm error")
        .to_string()
}
