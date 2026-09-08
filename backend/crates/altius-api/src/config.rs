use anyhow::Context;

#[derive(Debug, Clone)]
pub struct KeycloakConfig {
    /// e.g. `https://sso.example.com/realms/altius`
    pub issuer: String,
    /// Internal JWKS endpoint; defaults to `{issuer}/protocol/openid-connect/certs`.
    pub jwks_url_override: Option<String>,
    /// Token endpoint; defaults to `{issuer}/protocol/openid-connect/token`.
    pub token_url: String,
    /// Expected `aud` (client id, e.g. `altius-web` / `altius-mobile`).
    pub audience: String,
}

impl KeycloakConfig {
    pub fn jwks_url(&self) -> String {
        self.jwks_url_override
            .clone()
            .unwrap_or_else(|| format!("{}/protocol/openid-connect/certs", self.issuer))
    }
}

#[derive(Clone)]
pub struct Config {
    pub keycloak: KeycloakConfig,
    /// Which persistence backend serves the transactional routes.
    pub store_backend: StoreBackend,
    /// PostgreSQL connection string used when `store_backend` is `Postgres`.
    pub database_url: String,
    /// TypeDB database name used when `store_backend` is `Typedb`.
    pub typedb_database: String,
    pub google_maps_api_key: Option<String>,
    /// Google route mode: `directions` (default), `optimization`, or `routes`.
    pub google_route_mode: GoogleRouteMode,
    /// HMAC secret sealing paused agent runs. Without it the agent endpoints
    /// refuse to pause, because an unsigned run state is a forgeable one.
    pub agent_state_secret: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: String,
    /// Comma-separated allowed browser origins; empty = deny cross-origin.
    pub cors_origins: Vec<String>,
    /// Whether the resource-owner password grant login proxy is enabled.
    pub allow_password_grant: bool,
    /// Default org/hub and admin user to provision on startup.
    pub default_org_id: String,
    pub default_org_name: String,
    pub default_hub_id: String,
    pub default_hub_name: String,
    pub default_admin_sub: String,
    /// Service-account client for the Keycloak Admin API (user provisioning).
    /// Absent = provisioning disabled; the endpoint refuses rather than
    /// half-creating a user that exists in one system and not the other.
    pub keycloak_admin: Option<KeycloakAdminConfig>,
    /// Outbound SMS/WhatsApp gateway. Absent = notify endpoints refuse.
    pub notify: Option<NotifyConfig>,
    /// Firebase Cloud Messaging project id for FCM v1 HTTP push delivery.
    /// The client authenticates with Google Application Default Credentials.
    /// Absent = /api/v3/notify/push/send refuses.
    pub fcm_project_id: Option<String>,
    /// Optional path to a service account JSON. When absent, ADC is used.
    pub fcm_credentials_path: Option<String>,
    /// McEasy VSMS/TMS integration. Disabled when the API key is absent.
    pub mceasy: Option<MceasyConfig>,
}

/// Credentials for the Keycloak Admin REST API.
#[derive(Clone)]
pub struct KeycloakAdminConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Realm to create users in; defaults to the one parsed from the issuer.
    pub realm: String,
    /// Base URL without the `/realms/...` suffix, e.g. `https://sso.example.com`.
    pub base_url: String,
}

// Hand-written so a stray `{:?}` on Config can never print the secret.
impl std::fmt::Debug for KeycloakAdminConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeycloakAdminConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .field("realm", &self.realm)
            .field("base_url", &self.base_url)
            .finish()
    }
}

/// Generic HTTP messaging gateway (Twilio, Wassenger, or a regional provider).
#[derive(Clone)]
pub struct NotifyConfig {
    pub sms_url: Option<String>,
    pub whatsapp_url: Option<String>,
    pub token: String,
    pub sender: String,
}

/// McEasy VSMS/TMS integration settings.
#[derive(Clone)]
pub struct MceasyConfig {
    pub api_key: String,
    pub base_url: String,
    pub poll_seconds: u64,
    pub master_sync_seconds: u64,
    pub retention_hours: u64,
}

impl std::fmt::Debug for MceasyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MceasyConfig")
            .field("api_key", &"<redacted>")
            .field("base_url", &self.base_url)
            .field("poll_seconds", &self.poll_seconds)
            .field("master_sync_seconds", &self.master_sync_seconds)
            .field("retention_hours", &self.retention_hours)
            .finish()
    }
}

impl std::fmt::Debug for NotifyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotifyConfig")
            .field("sms_url", &self.sms_url)
            .field("whatsapp_url", &self.whatsapp_url)
            .field("token", &"<redacted>")
            .field("sender", &self.sender)
            .finish()
    }
}

// Hand-written so a stray `{:?}` on Config can never print the database URL
// (which contains the Postgres password) or any integration API key.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("keycloak", &self.keycloak)
            .field("store_backend", &self.store_backend)
            .field("database_url", &"<redacted>")
            .field("typedb_database", &self.typedb_database)
            .field(
                "google_maps_api_key",
                &self.google_maps_api_key.as_ref().map(|_| "<redacted>"),
            )
            .field("google_route_mode", &self.google_route_mode)
            .field(
                "agent_state_secret",
                &self.agent_state_secret.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "openrouter_api_key",
                &self.openrouter_api_key.as_ref().map(|_| "<redacted>"),
            )
            .field("openrouter_model", &self.openrouter_model)
            .field("cors_origins", &self.cors_origins)
            .field("allow_password_grant", &self.allow_password_grant)
            .field("default_org_id", &self.default_org_id)
            .field("default_org_name", &self.default_org_name)
            .field("default_hub_id", &self.default_hub_id)
            .field("default_hub_name", &self.default_hub_name)
            .field("default_admin_sub", &self.default_admin_sub)
            .field("keycloak_admin", &self.keycloak_admin)
            .field("notify", &self.notify)
            .field("fcm_project_id", &self.fcm_project_id)
            .field("fcm_credentials_path", &self.fcm_credentials_path)
            .field("mceasy", &self.mceasy)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoreBackend {
    /// PostgreSQL is the default transactional source of truth.
    #[default]
    Postgres,
    /// TypeDB remains available for relation/inference experiments.
    Typedb,
}

impl StoreBackend {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "typedb" => Self::Typedb,
            _ => Self::Postgres,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GoogleRouteMode {
    #[default]
    Directions,
    Optimization,
    Routes,
}

impl GoogleRouteMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "optimization" => Self::Optimization,
            "routes" => Self::Routes,
            _ => Self::Directions,
        }
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let issuer = std::env::var("KEYCLOAK_ISSUER")
            .context("KEYCLOAK_ISSUER required (e.g. https://host/realms/altius)")?;
        Ok(Self {
            keycloak: KeycloakConfig {
                issuer: issuer.clone(),
                jwks_url_override: std::env::var("KEYCLOAK_JWKS_URL").ok(),
                token_url: std::env::var("KEYCLOAK_TOKEN_URL")
                    .unwrap_or_else(|_| format!("{issuer}/protocol/openid-connect/token")),
                audience: std::env::var("KEYCLOAK_AUDIENCE").unwrap_or_else(|_| "altius".into()),
            },
            store_backend: StoreBackend::from_str(
                &std::env::var("STORE_BACKEND").unwrap_or_else(|_| "postgres".into()),
            ),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/altius".into()),
            typedb_database: std::env::var("TYPEDB_DATABASE").unwrap_or_else(|_| "altius".into()),
            google_maps_api_key: std::env::var("GOOGLE_MAPS_API_KEY").ok(),
            google_route_mode: GoogleRouteMode::from_str(
                &std::env::var("GOOGLE_ROUTE_MODE").unwrap_or_else(|_| "directions".into()),
            ),
            agent_state_secret: std::env::var("AGENT_STATE_SECRET").ok(),
            openrouter_api_key: std::env::var("OPENROUTER_API_KEY").ok(),
            openrouter_model: std::env::var("OPENROUTER_MODEL")
                .unwrap_or_else(|_| "openai/gpt-4o-mini".into()),
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            allow_password_grant: std::env::var("ALLOW_PASSWORD_GRANT")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            default_org_id: std::env::var("DEFAULT_ORG_ID").unwrap_or_else(|_| "altius".into()),
            default_org_name: std::env::var("DEFAULT_ORG_NAME").unwrap_or_else(|_| "Altius".into()),
            default_hub_id: std::env::var("DEFAULT_HUB_ID").unwrap_or_else(|_| "jakarta".into()),
            default_hub_name: std::env::var("DEFAULT_HUB_NAME")
                .unwrap_or_else(|_| "Jakarta".into()),
            default_admin_sub: std::env::var("DEFAULT_ADMIN_SUB")
                .unwrap_or_else(|_| "admin".into()),
            keycloak_admin: match (
                std::env::var("KEYCLOAK_ADMIN_CLIENT_ID").ok(),
                std::env::var("KEYCLOAK_ADMIN_CLIENT_SECRET").ok(),
            ) {
                (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
                    let (base_url, realm) = split_issuer(&issuer);
                    Some(KeycloakAdminConfig {
                        client_id: id,
                        client_secret: secret,
                        realm: std::env::var("KEYCLOAK_ADMIN_REALM").unwrap_or(realm),
                        base_url: std::env::var("KEYCLOAK_ADMIN_BASE_URL").unwrap_or(base_url),
                    })
                }
                _ => None,
            },
            notify: std::env::var("NOTIFY_TOKEN")
                .ok()
                .filter(|t| !t.is_empty())
                .map(|token| NotifyConfig {
                    sms_url: std::env::var("NOTIFY_SMS_URL")
                        .ok()
                        .filter(|u| !u.is_empty()),
                    whatsapp_url: std::env::var("NOTIFY_WHATSAPP_URL")
                        .ok()
                        .filter(|u| !u.is_empty()),
                    token,
                    sender: std::env::var("NOTIFY_SENDER").unwrap_or_default(),
                }),
            fcm_project_id: std::env::var("FCM_PROJECT_ID")
                .ok()
                .filter(|s| !s.is_empty()),
            fcm_credentials_path: std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .ok()
                .filter(|s| !s.is_empty()),
            mceasy: std::env::var("MCEASY_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .map(|api_key| MceasyConfig {
                    api_key,
                    base_url: std::env::var("MCEASY_BASE_URL")
                        .unwrap_or_else(|_| "https://vsms-v2-public.mceasy.com".into()),
                    poll_seconds: std::env::var("MCEASY_POLL_SECONDS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(60),
                    master_sync_seconds: std::env::var("MCEASY_MASTER_SYNC_SECONDS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(3600),
                    retention_hours: std::env::var("MCEASY_RETENTION_HOURS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(72),
                }),
        })
    }
}

/// Split `https://host/realms/altius` into (`https://host`, `altius`).
///
/// Falls back to the whole issuer and an empty realm when the shape is
/// unexpected, so a misconfigured issuer surfaces as a failed admin call
/// rather than a silently wrong URL.
fn split_issuer(issuer: &str) -> (String, String) {
    match issuer.rsplit_once("/realms/") {
        Some((base, realm)) => (base.to_string(), realm.trim_end_matches('/').to_string()),
        None => (issuer.trim_end_matches('/').to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::split_issuer;

    #[test]
    fn issuer_splits_into_base_and_realm() {
        assert_eq!(
            split_issuer("https://sso.example.com/realms/altius"),
            ("https://sso.example.com".into(), "altius".into())
        );
        // Trailing slash must not leak into the realm name.
        assert_eq!(
            split_issuer("https://sso.example.com/realms/altius/").1,
            "altius"
        );
        // Unexpected shape: no realm rather than a guessed one.
        assert_eq!(split_issuer("https://sso.example.com").1, "");
    }
}
