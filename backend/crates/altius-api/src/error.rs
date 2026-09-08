use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("not found")]
    NotFound,
    #[error("service unavailable: {0}")]
    Unavailable(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (code, msg) = match &self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Self::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Unavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone()),
            Self::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };
        (code, Json(json!({ "error": { "code": code.as_u16(), "message": msg } })))
            .into_response()
    }
}

impl ApiError {
    /// An upstream dependency failed. Logs the real error server-side and
    /// returns a fixed, client-safe message.
    ///
    /// Never format an upstream transport error into a client-visible string
    /// directly: `reqwest::Error`'s `Display` embeds the full request URL, and
    /// our outbound URLs carry credentials in the query string (the Google
    /// Maps `key` parameter), so `format!("{e}")` hands the API key to whoever
    /// triggered the failure.
    pub fn upstream(what: &'static str, e: impl std::fmt::Display) -> Self {
        tracing::error!(upstream = what, error = %e, "upstream request failed");
        Self::Unavailable(format!("{what} unavailable"))
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
