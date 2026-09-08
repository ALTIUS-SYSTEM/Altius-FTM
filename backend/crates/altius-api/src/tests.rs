use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::AppState;
use crate::auth::Jwks;
use crate::config::{Config, KeycloakConfig};

fn test_state() -> Arc<AppState> {
    let config = Config {
        keycloak_admin: None,
        notify: None,
        mceasy: None,
        fcm_project_id: None,
        fcm_credentials_path: None,
        keycloak: KeycloakConfig {
            issuer: "https://sso.example.com/realms/test".into(),
            jwks_url_override: None,
            token_url: "https://sso.example.com/realms/test/protocol/openid-connect/token".into(),
            audience: "altius".into(),
        },
        store_backend: crate::config::StoreBackend::Postgres,
        database_url: String::new(),
        typedb_database: "test".into(),
        google_maps_api_key: None,
        google_route_mode: crate::config::GoogleRouteMode::Directions,
        agent_state_secret: None,
        openrouter_api_key: None,
        openrouter_model: "test-model".into(),
        cors_origins: vec![],
        allow_password_grant: false,
        default_org_id: "altius".into(),
        default_org_name: "Altius".into(),
        default_hub_id: "jakarta".into(),
        default_hub_name: "Jakarta".into(),
        default_admin_sub: "admin".into(),
    };
    Arc::new(AppState {
        mceasy_client: None,
        fcm_client: None,
        metrics: Arc::new(crate::metrics::Metrics::new().unwrap()),
        jwks: Jwks::new(config.keycloak.clone()),
        config,
        http: reqwest::Client::new(),
        store: None,
    })
}

#[tokio::test]
async fn health_is_public() {
    let app = crate::routes::router().with_state(test_state());
    let res = app
        .oneshot(Request::get("/api/v3/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn tasks_requires_bearer() {
    let app = crate::routes::router().with_state(test_state());
    let res = app
        .oneshot(Request::get("/api/v3/tasks").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn events_requires_bearer() {
    let app = crate::routes::router().with_state(test_state());
    let res = app
        .oneshot(
            Request::post("/api/v3/events")
                .header("content-type", "application/json")
                .body(Body::from("[]"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn garbage_token_rejected() {
    let app = crate::routes::router().with_state(test_state());
    let res = app
        .oneshot(
            Request::get("/api/v3/tasks")
                .header("authorization", "Bearer not.a.jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
