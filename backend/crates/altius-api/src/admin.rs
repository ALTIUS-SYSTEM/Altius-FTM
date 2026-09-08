//! Keycloak Admin REST client — user provisioning.
//!
//! Uses a confidential service-account client (client_credentials grant), not
//! an operator's password. The secret is server-side only and never appears in
//! a client-visible error: every upstream failure goes through
//! `ApiError::upstream`, which logs the detail and returns a fixed string.

use serde_json::json;

use crate::config::KeycloakAdminConfig;
use crate::error::{ApiError, ApiResult};

pub struct AdminClient<'a> {
    http: &'a reqwest::Client,
    config: &'a KeycloakAdminConfig,
}

/// A newly provisioned Keycloak user.
pub struct ProvisionedUser {
    pub subject: String,
    /// One-time password, flagged `temporary` so Keycloak forces a change at
    /// first login. Returned to the calling admin once and never stored.
    pub temporary_password: String,
}

impl<'a> AdminClient<'a> {
    pub fn new(http: &'a reqwest::Client, config: &'a KeycloakAdminConfig) -> Self {
        Self { http, config }
    }

    /// Service-account access token for the Admin API.
    async fn token(&self) -> ApiResult<String> {
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.config.base_url, self.config.realm
        );
        let res = self
            .http
            .post(&url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
            ])
            .send()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?;
        if !res.status().is_success() {
            tracing::error!(status = %res.status(), "keycloak admin token request failed");
            return Err(ApiError::Unavailable(
                "identity provider unavailable".into(),
            ));
        }
        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?;
        body.get("access_token")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                ApiError::Internal(anyhow::anyhow!("admin token response had no access_token"))
            })
    }

    /// Create a realm user with a temporary password.
    ///
    /// Returns `Conflict` when the username or email is already taken, so the
    /// caller can report that instead of a generic failure.
    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        display_name: &str,
        realm_roles: &[String],
    ) -> ApiResult<ProvisionedUser> {
        let token = self.token().await?;
        let users_url = format!(
            "{}/admin/realms/{}/users",
            self.config.base_url, self.config.realm
        );
        let password = temporary_password();
        let (first, last) = split_name(display_name);

        let res = self
            .http
            .post(&users_url)
            .bearer_auth(&token)
            .json(&json!({
                "username": username,
                "email": email,
                "firstName": first,
                "lastName": last,
                "enabled": true,
                "emailVerified": false,
                "credentials": [{
                    "type": "password",
                    "value": password,
                    // Forces a change at first login: the provisioning password
                    // is a handover token, not a durable credential.
                    "temporary": true,
                }],
            }))
            .send()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?;

        if res.status() == reqwest::StatusCode::CONFLICT {
            return Err(ApiError::Conflict(
                "username or email already exists".into(),
            ));
        }
        if !res.status().is_success() {
            tracing::error!(status = %res.status(), "keycloak user creation failed");
            return Err(ApiError::Unavailable(
                "identity provider unavailable".into(),
            ));
        }

        // Keycloak returns the new id only in the Location header.
        let subject = res
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|loc| loc.rsplit('/').next())
            .map(str::to_string)
            .ok_or_else(|| {
                ApiError::Internal(anyhow::anyhow!("keycloak did not return a user id"))
            })?;

        if !realm_roles.is_empty() {
            self.assign_realm_roles(&token, &subject, realm_roles)
                .await?;
        }

        Ok(ProvisionedUser {
            subject,
            temporary_password: password,
        })
    }

    async fn assign_realm_roles(
        &self,
        token: &str,
        subject: &str,
        roles: &[String],
    ) -> ApiResult<()> {
        let mut payload = Vec::new();
        for role in roles {
            let url = format!(
                "{}/admin/realms/{}/roles/{role}",
                self.config.base_url, self.config.realm
            );
            let res = self
                .http
                .get(&url)
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| ApiError::upstream("identity provider", e))?;
            if !res.status().is_success() {
                return Err(ApiError::BadRequest(format!("unknown realm role: {role}")));
            }
            payload.push(
                res.json::<serde_json::Value>()
                    .await
                    .map_err(|e| ApiError::upstream("identity provider", e))?,
            );
        }
        let url = format!(
            "{}/admin/realms/{}/users/{subject}/role-mappings/realm",
            self.config.base_url, self.config.realm
        );
        let res = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?;
        if !res.status().is_success() {
            tracing::error!(status = %res.status(), "realm role assignment failed");
            return Err(ApiError::Unavailable(
                "identity provider unavailable".into(),
            ));
        }
        Ok(())
    }

    /// Replace a user's realm roles with exactly `roles`.
    ///
    /// This is what "permission management" means here: the API authorizes on
    /// Keycloak realm roles, so roles are the only grant that actually
    /// enforces anything. A separate permission store would be a second
    /// authority that no server component consults.
    pub async fn set_realm_roles(&self, subject: &str, roles: &[String]) -> ApiResult<()> {
        let token = self.token().await?;
        let mapping_url = format!(
            "{}/admin/realms/{}/users/{subject}/role-mappings/realm",
            self.config.base_url, self.config.realm
        );

        // Remove the roles this system manages, then add the requested set —
        // leaves unrelated realm roles (granted for other apps) untouched.
        let current: Vec<serde_json::Value> = self
            .http
            .get(&mapping_url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?
            .json()
            .await
            .map_err(|e| ApiError::upstream("identity provider", e))?;
        let managed: Vec<serde_json::Value> = current
            .into_iter()
            .filter(|r| {
                r.get("name")
                    .and_then(|n| n.as_str())
                    .is_some_and(|n| MANAGED_ROLES.contains(&n))
            })
            .collect();
        if !managed.is_empty() {
            let res = self
                .http
                .delete(&mapping_url)
                .bearer_auth(&token)
                .json(&managed)
                .send()
                .await
                .map_err(|e| ApiError::upstream("identity provider", e))?;
            if !res.status().is_success() {
                tracing::error!(status = %res.status(), "realm role removal failed");
                return Err(ApiError::Unavailable(
                    "identity provider unavailable".into(),
                ));
            }
        }
        if !roles.is_empty() {
            self.assign_realm_roles(&token, subject, roles).await?;
        }
        Ok(())
    }
}

/// Realm roles this API understands and therefore manages. `integration` is
/// for service accounts, not people — `create_user` should not offer it.
pub const MANAGED_ROLES: [&str; 6] = [
    "super-admin",
    "admin",
    "supervisor",
    "lead",
    "driver",
    "integration",
];

/// One-time password from the OS CSPRNG (uuid v4 is CSPRNG-backed).
fn temporary_password() -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{a}{b}")
}

/// Split a display name into Keycloak's first/last fields.
fn split_name(display_name: &str) -> (&str, &str) {
    match display_name.trim().split_once(' ') {
        Some((first, rest)) => (first, rest.trim()),
        None => (display_name.trim(), ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_split_on_the_first_space() {
        assert_eq!(split_name("Adi Pratama"), ("Adi", "Pratama"));
        assert_eq!(
            split_name("  Nadia  Putri Wibowo "),
            ("Nadia", "Putri Wibowo")
        );
        assert_eq!(split_name("Sari"), ("Sari", ""));
    }

    #[test]
    fn temporary_passwords_are_long_and_unique() {
        let a = temporary_password();
        let b = temporary_password();
        assert_eq!(a.len(), 64, "two v4 UUIDs, hyphens stripped");
        assert_ne!(a, b, "each provisioning must mint a fresh password");
    }
}
