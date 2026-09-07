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
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: String,
    /// Comma-separated allowed browser origins; empty = deny cross-origin.
    pub cors_origins: Vec<String>,
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
        })
    }
}
