use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, store};
use crate::AppState;
use crate::error::{ApiError, ApiResult};

#[derive(serde::Deserialize)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct RefreshRequest {
    refresh_token: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/auth/login", post(login))
        .route("/api/v3/auth/refresh", post(refresh))
        .route("/api/v3/auth/me", get(me))
}

async fn login(
    State(s): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<Value>> {
    if !s.config.allow_password_grant {
        return Err(ApiError::Unavailable(
            "resource-owner password grant is disabled".into(),
        ));
    }
    if req.username.is_empty() || req.password.is_empty() {
        return Err(ApiError::BadRequest(
            "username and password required".into(),
        ));
    }
    let params = [
        ("grant_type", "password"),
        ("client_id", s.config.keycloak.audience.as_str()),
        ("username", req.username.as_str()),
        ("password", req.password.as_str()),
    ];
    let token = exchange_tokens(&s.http, &s.config.keycloak.token_url, &params).await?;
    Ok(Json(token))
}

async fn refresh(
    State(s): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> ApiResult<Json<Value>> {
    // Refresh is the continuation of the password grant; leaving it open while
    // `login` is gated would let a captured refresh token keep minting access
    // tokens after the grant was switched off.
    if !s.config.allow_password_grant {
        return Err(ApiError::Unavailable(
            "resource-owner password grant is disabled".into(),
        ));
    }
    if req.refresh_token.is_empty() {
        return Err(ApiError::BadRequest("refresh_token required".into()));
    }
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", s.config.keycloak.audience.as_str()),
        ("refresh_token", req.refresh_token.as_str()),
    ];
    let token = exchange_tokens(&s.http, &s.config.keycloak.token_url, &params).await?;
    Ok(Json(token))
}

async fn me(State(s): State<Arc<AppState>>, principal: AuthUser) -> ApiResult<Json<Value>> {
    let (org, hub) = store(&s)?
        .organization_and_hub_of(&principal.subject)
        .await
        .map_err(ApiError::Internal)?
        .unwrap_or_default();
    Ok(Json(json!({
        "data": {
            "subject": principal.subject,
            "roles": principal.roles,
            "organization": org,
            "hub": hub,
        }
    })))
}

/// Exchange a grant for tokens at the configured identity provider.
async fn exchange_tokens(
    http: &reqwest::Client,
    token_url: &str,
    params: &[(&str, &str)],
) -> ApiResult<Value> {
    let resp = http
        .post(token_url)
        .form(params)
        .send()
        .await
        .map_err(|e| ApiError::upstream("identity provider", e))?;
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
    Ok(json!({
        "accessToken": token.get("access_token").and_then(Value::as_str),
        "refreshToken": token.get("refresh_token").and_then(Value::as_str),
        "expiresIn": token.get("expires_in").and_then(Value::as_u64),
        "tokenType": token.get("token_type").and_then(Value::as_str),
    }))
}
