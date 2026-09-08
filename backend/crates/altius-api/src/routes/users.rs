use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{AuthUser, org_of, require_admin, require_staff_or_integration, store};
use crate::AppState;
use crate::admin::{AdminClient, MANAGED_ROLES};
use crate::error::{ApiError, ApiResult};

#[derive(serde::Deserialize)]
pub(crate) struct CreateUserRequest {
    username: String,
    email: String,
    display_name: String,
    #[serde(default)]
    realm_roles: Vec<String>,
    /// Hub to assign; must belong to the caller's organization. Defaults to
    /// the caller's own hub.
    #[serde(default)]
    hub_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct RolesRequest {
    roles: Vec<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v3/users", get(list_users).post(create_user_account))
        .route("/api/v3/users/{sub}/roles", put(set_user_roles))
        .route("/api/v3/drivers", get(list_drivers))
}

async fn list_users(State(s): State<Arc<AppState>>, principal: AuthUser) -> ApiResult<Json<Value>> {
    require_staff_or_integration(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let users = store(&s)?
        .users_for_org(&org)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        json!({ "data": users, "meta": { "organization": org } }),
    ))
}

async fn list_drivers(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
) -> ApiResult<Json<Value>> {
    require_staff_or_integration(&principal)?;
    let org = org_of(&s, &principal.subject).await?;
    let drivers = store(&s)?
        .drivers_for_org(&org)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        json!({ "data": drivers, "meta": { "organization": org } }),
    ))
}

async fn create_user_account(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Json(req): Json<CreateUserRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let admin_cfg = s
        .config
        .keycloak_admin
        .as_ref()
        .ok_or_else(|| ApiError::Unavailable("user provisioning is not configured".into()))?;

    if req.username.trim().is_empty() || req.email.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "username and email are required".into(),
        ));
    }
    if req.display_name.trim().len() > 200 {
        return Err(ApiError::BadRequest("display name is too long".into()));
    }
    // Only roles this system understands; an arbitrary realm role would be
    // granted here and mean nothing to the API's own authorization.
    // One allowlist, shared with `set_user_roles`. A local copy drifted once
    // already: it omitted "super-admin" while the web form offered it, so the
    // dropdown produced a role the API rejected.
    // `integration` is M2M-only — bind via POST /integrations, never here.
    if req.realm_roles.iter().any(|r| r == "integration") {
        return Err(ApiError::BadRequest(
            "integration role is for service accounts; use POST /api/v3/integrations".into(),
        ));
    }
    if let Some(bad) = req
        .realm_roles
        .iter()
        .find(|r| !MANAGED_ROLES.contains(&r.as_str()))
    {
        return Err(ApiError::BadRequest(format!("unsupported role: {bad}")));
    }

    // Scope comes from the caller's token, never the request body.
    let (org, own_hub) = store(&s)?
        .organization_and_hub_of(&principal.subject)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Forbidden)?;
    let hub = req.hub_id.unwrap_or(own_hub);

    let created = AdminClient::new(&s.http, admin_cfg)
        .create_user(
            req.username.trim(),
            req.email.trim(),
            req.display_name.trim(),
            &req.realm_roles,
        )
        .await?;

    // The identity exists now; if the membership write fails the account would
    // be a ghost — say so explicitly rather than reporting success.
    store(&s)?
        .provision_user(
            &org,
            &hub,
            &created.subject,
            req.display_name.trim(),
            req.realm_roles.first().map(String::as_str).unwrap_or("driver"),
            Some(req.email.trim()),
        )
        .await
        .map_err(|e| {
            tracing::error!(subject = %created.subject, error = %e, "keycloak user created but membership write failed");
            ApiError::Internal(anyhow::anyhow!(
                "user created in the identity provider but not linked to the organization"
            ))
        })?;

    Ok(Json(json!({
        "data": {
            "subject": created.subject,
            "organization": org,
            "hub": hub,
            // Shown once, to the admin who created the account. Keycloak marks
            // it temporary, so the user must change it at first login.
            "temporaryPassword": created.temporary_password,
        }
    })))
}

/// Set a user's realm roles — this system's only real permission control.
///
/// Roles live in Keycloak because that is what the token carries and what
/// `AuthUser` enforces. A separate permission table would be a second
/// authority that nothing consults.
async fn set_user_roles(
    State(s): State<Arc<AppState>>,
    principal: AuthUser,
    Path(subject): Path<String>,
    Json(req): Json<RolesRequest>,
) -> ApiResult<Json<Value>> {
    require_admin(&principal)?;
    let admin_cfg = s
        .config
        .keycloak_admin
        .as_ref()
        .ok_or_else(|| ApiError::Unavailable("user provisioning is not configured".into()))?;
    if req.roles.iter().any(|r| r == "integration") {
        return Err(ApiError::BadRequest(
            "integration role is for service accounts; use POST /api/v3/integrations".into(),
        ));
    }
    if let Some(bad) = req
        .roles
        .iter()
        .find(|r| !MANAGED_ROLES.contains(&r.as_str()))
    {
        return Err(ApiError::BadRequest(format!("unsupported role: {bad}")));
    }
    if req.roles.iter().collect::<HashSet<_>>().len() != req.roles.len() {
        return Err(ApiError::BadRequest("duplicate roles".into()));
    }
    // An admin may only change roles for someone in their own organization.
    let org = org_of(&s, &principal.subject).await?;
    if !store(&s)?
        .user_in_org(&org, &subject)
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::NotFound);
    }
    // Removing your own admin role locks you (and possibly the tenant) out.
    if subject == principal.subject && !req.roles.iter().any(|r| r == "admin") {
        return Err(ApiError::BadRequest(
            "you cannot remove your own admin role".into(),
        ));
    }
    AdminClient::new(&s.http, admin_cfg)
        .set_realm_roles(&subject, &req.roles)
        .await?;
    // Keycloak is the authority; this only refreshes the roster cache, so a
    // failure here must not fail the request that already changed the grant.
    if let Some(primary) = req.roles.first()
        && let Err(e) = store(&s)?.set_cached_role(&org, &subject, primary).await
    {
        tracing::warn!(%subject, error = %e, "roles changed in Keycloak but roster cache not refreshed");
    }
    Ok(Json(
        json!({ "data": { "subject": subject, "roles": req.roles } }),
    ))
}
