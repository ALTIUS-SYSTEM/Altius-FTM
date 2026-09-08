use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, require_staff, store};
use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::notify::{Channel, Notifier};

#[derive(serde::Deserialize)]
pub(crate) struct NotifyRequest {
    to: String,
    message: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct RegisterPushRequest {
    device_id: String,
    token: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct SendPushRequest {
    to_subject: String,
    title: String,
    body: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/notify/sms", post(notify_sms))
        .route("/api/v3/notify/whatsapp", post(notify_whatsapp))
        .route("/api/v3/notify/push", post(register_push))
        .route("/api/v3/notify/push/send", post(send_push))
}

async fn notify_sms(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<NotifyRequest>,
) -> ApiResult<Json<Value>> {
    send_message(&s, &principal, Channel::Sms, req).await
}

async fn notify_whatsapp(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<NotifyRequest>,
) -> ApiResult<Json<Value>> {
    send_message(&s, &principal, Channel::WhatsApp, req).await
}

async fn register_push(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<RegisterPushRequest>,
) -> ApiResult<Json<Value>> {
    if req.device_id.trim().is_empty() || req.token.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "device_id and token are required".into(),
        ));
    }
    store(&s)?
        .register_push_token(&principal.subject, req.device_id.trim(), req.token.trim())
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(json!({ "data": { "registered": true } })))
}

async fn send_push(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<SendPushRequest>,
) -> ApiResult<Json<Value>> {
    // Push is a staff action: it delivers to a driver's device and can be
    // used for harassment if ungated.
    require_staff(&principal)?;
    if req.to_subject.trim().is_empty() || req.title.trim().is_empty() || req.body.trim().is_empty()
    {
        return Err(ApiError::BadRequest(
            "to_subject, title and body are required".into(),
        ));
    }
    // Cross-tenant guard: a staff member may only push to a driver in their
    // own organization. Without this, knowing a subject lets you reach any
    // tenant's devices.
    let org = super::org_of(&s, &principal.subject).await?;
    if !store(&s)?
        .user_in_org(&org, req.to_subject.trim())
        .await
        .map_err(ApiError::Internal)?
    {
        // Match the existing "no tokens" wording so the 404 does not leak
        // whether the subject exists in another tenant.
        return Err(ApiError::Unavailable(
            "driver has no registered push tokens".into(),
        ));
    }
    let tokens = store(&s)?
        .push_tokens_for_user(&req.to_subject)
        .await
        .map_err(ApiError::Internal)?;
    if tokens.is_empty() {
        return Err(ApiError::Unavailable(
            "driver has no registered push tokens".into(),
        ));
    }
    // FCM v1 uses Google ADC. The key used to be a legacy FCM server key.
    let Some(client) = s.fcm_client.clone() else {
        return Err(ApiError::Unavailable("FCM not configured".into()));
    };
    let deliveries = client.send_batch(&tokens, &req.title, &req.body).await?;
    s.metrics.fcm_send_total.inc_by(tokens.len() as f64);
    let failed = deliveries.iter().filter(|d| d.error.is_some()).count();
    Ok(Json(json!({
        "data": {
            "queued": tokens.len(),
            "delivered": deliveries.len() - failed,
            "failed": failed,
        }
    })))
}

async fn send_message(
    s: &Arc<AppState>,
    principal: &AuthUser,
    channel: Channel,
    req: NotifyRequest,
) -> ApiResult<Json<Value>> {
    // Sending on the org's account costs money and reaches real phones, so it
    // is staff-only — a driver cannot use the gateway as a relay.
    require_staff(principal)?;
    let cfg = s
        .config
        .notify
        .as_ref()
        .ok_or_else(|| ApiError::Unavailable("messaging gateway is not configured".into()))?;
    let recipient = Notifier::new(&s.http, cfg)
        .send(channel, &req.to, &req.message)
        .await?;
    Ok(Json(
        json!({ "data": { "to": recipient, "status": "queued" } }),
    ))
}
