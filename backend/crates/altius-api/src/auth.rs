//! Keycloak OIDC Bearer validation — RS256 against realm JWKS.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
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
    /// Last outbound attempt, successful or not. Rate-limits refetches driven
    /// by unknown `kid`s, which come from the *unverified* token header.
    attempted_at: Instant,
}

const JWKS_TTL: Duration = Duration::from_secs(300);
/// Minimum gap between unknown-`kid`-triggered refetches.
const JWKS_REFRESH_COOLDOWN: Duration = Duration::from_secs(30);

impl Jwks {
    pub fn new(config: KeycloakConfig) -> Self {
        let stale = Instant::now() - JWKS_TTL;
        Self {
            inner: Arc::new(RwLock::new(JwksState {
                keys: jsonwebtoken::jwk::JwkSet { keys: vec![] },
                fetched_at: stale,
                attempted_at: stale,
            })),
            config,
            // JWKS fetches sit on the request path for every bearer token.
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .connect_timeout(Duration::from_secs(5))
                .build()
                .expect("build jwks client"),
        }
    }

    async fn refresh(&self) -> Result<(), ApiError> {
        self.inner.write().await.attempted_at = Instant::now();
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
            let stale = guard.fetched_at.elapsed() > JWKS_TTL;
            let unknown_kid = !guard
                .keys
                .keys
                .iter()
                .any(|k| k.common.key_id.as_deref() == Some(kid));
            // `kid` is attacker-controlled and read before any signature check,
            // so an unknown one must not buy an uncached outbound fetch on every
            // request. Honour the TTL always; honour unknown-kid only outside
            // the cooldown.
            stale || (unknown_kid && guard.attempted_at.elapsed() > JWKS_REFRESH_COOLDOWN)
        };
        if need_refresh {
            self.refresh().await?;
        }
        let guard = self.inner.read().await;
        guard.keys.find(kid).cloned().ok_or(ApiError::Unauthorized)
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
        // would be accepted the moment it is minted. Keep nbf optional (Keycloak
        // often omits it); jsonwebtoken ≥ 10.3 rejects a malformed typed nbf
        // instead of treating FailedToParse as absent (CVE-2026-25537).
        validation.validate_nbf = true;
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);

        let data =
            decode::<Claims>(token, &key, &validation).map_err(|_| ApiError::Unauthorized)?;
        let claims = data.claims;

        let roles: Vec<Role> = claims
            .realm_access
            .roles
            .iter()
            .filter_map(|r| match r.as_str() {
                "super-admin" => Some(Role::SuperAdmin),
                "admin" => Some(Role::Admin),
                "supervisor" => Some(Role::Supervisor),
                "lead" => Some(Role::Lead),
                "driver" => Some(Role::Driver),
                "integration" => Some(Role::Integration),
                _ => None,
            })
            .collect();

        // Unrecognised realm roles are dropped above, so any realm token —
        // including a service account or a self-registered user with no fleet
        // role — would otherwise arrive as an authenticated principal that
        // every `has_role` check answers "false" for, i.e. a full reader of
        // every route that only gates on Driver. Require a known role.
        if roles.is_empty() {
            return Err(ApiError::Forbidden);
        }

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
