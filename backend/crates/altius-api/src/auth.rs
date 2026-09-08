//! Keycloak OIDC Bearer validation — RS256 against realm JWKS.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::config::KeycloakConfig;
use crate::error::ApiError;
use altius_core::{Principal, Role};

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    realm_access: RealmAccess,
    #[serde(default)]
    organization_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RealmAccess {
    #[serde(default)]
    roles: Vec<String>,
}

#[derive(Clone)]
pub struct Jwks {
    inner: Arc<RwLock<JwksState>>,
    config: KeycloakConfig,
    http: reqwest::Client,
}

struct JwksState {
    keys: jsonwebtoken::jwk::JwkSet,
    fetched_at: Instant,
}

const JWKS_TTL: Duration = Duration::from_secs(300);

impl Jwks {
    pub fn new(config: KeycloakConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(JwksState {
                keys: jsonwebtoken::jwk::JwkSet { keys: vec![] },
                fetched_at: Instant::now() - JWKS_TTL,
            })),
            config,
            http: reqwest::Client::new(),
        }
    }

    async fn refresh(&self) -> Result<(), ApiError> {
        let set: jsonwebtoken::jwk::JwkSet = self
            .http
            .get(self.config.jwks_url())
            .send()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?
            .json()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?;
        let mut guard = self.inner.write().await;
        guard.keys = set;
        guard.fetched_at = Instant::now();
        Ok(())
    }

    async fn find_key(&self, kid: &str) -> Result<jsonwebtoken::jwk::Jwk, ApiError> {
        let need_refresh = {
            let guard = self.inner.read().await;
            guard.fetched_at.elapsed() > JWKS_TTL
                || !guard.keys.keys.iter().any(|k| k.common.key_id.as_deref() == Some(kid))
        };
        if need_refresh {
            self.refresh().await?;
        }
        let guard = self.inner.read().await;
        guard
            .keys
            .find(kid)
            .cloned()
            .ok_or(ApiError::Unauthorized)
    }

    /// Validate a Bearer token and resolve the principal.
    pub async fn validate(&self, token: &str) -> Result<Principal, ApiError> {
        let header = decode_header(token).map_err(|_| ApiError::Unauthorized)?;
        let kid = header.kid.ok_or(ApiError::Unauthorized)?;

        let jwk = self.find_key(&kid).await?;
        let key = DecodingKey::from_jwk(&jwk).map_err(|_| ApiError::Unauthorized)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.validate_exp = true;
        // jsonwebtoken defaults validate_nbf to false, so a post-dated token
        // would be accepted the moment it is minted.
        validation.validate_nbf = true;
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);

        let data = decode::<Claims>(token, &key, &validation)
            .map_err(|_| ApiError::Unauthorized)?;
        let claims = data.claims;

        let roles = claims
            .realm_access
            .roles
            .iter()
            .filter_map(|r| match r.as_str() {
                "admin" => Some(Role::Admin),
                "supervisor" => Some(Role::Supervisor),
                "lead" => Some(Role::Lead),
                "driver" => Some(Role::Driver),
                _ => None,
            })
            .collect();

        Ok(Principal {
            subject: claims.sub,
            roles,
            organization_id: claims.organization_id,
        })
    }
}

fn bearer(parts: &Parts) -> Result<&str, ApiError> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)
}

/// Axum extractor — wraps `Principal` (foreign type) for the orphan rule.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Principal);

impl std::ops::Deref for AuthUser {
    type Target = Principal;
    fn deref(&self) -> &Principal {
        &self.0
    }
}

impl FromRequestParts<Arc<crate::AppState>> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<crate::AppState>,
    ) -> Result<Self, Self::Rejection> {
        state.jwks.validate(bearer(parts)?).await.map(Self)
    }
}
