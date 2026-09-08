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

#[derive(Debug, Clone)]
pub struct Config {
    pub keycloak: KeycloakConfig,
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
            default_org_name: std::env::var("DEFAULT_ORG_NAME")
                .unwrap_or_else(|_| "Altius".into()),
            default_hub_id: std::env::var("DEFAULT_HUB_ID").unwrap_or_else(|_| "jakarta".into()),
            default_hub_name: std::env::var("DEFAULT_HUB_NAME")
                .unwrap_or_else(|_| "Jakarta".into()),
            default_admin_sub: std::env::var("DEFAULT_ADMIN_SUB")
                .unwrap_or_else(|_| "admin".into()),
        })
    }
}
