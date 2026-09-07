use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::auth::Jwks;
use crate::config::{Config, KeycloakConfig};
use crate::AppState;

fn test_state() -> Arc<AppState> {
    let config = Config {
        keycloak: KeycloakConfig {
            issuer: "https://sso.example.com/realms/test".into(),
            jwks_url_override: None,
            token_url: "https://sso.example.com/realms/test/protocol/openid-connect/token".into(),
            audience: "altius".into(),
        },
        typedb_database: "test".into(),
        google_maps_api_key: None,
        google_route_mode: crate::config::GoogleRouteMode::Directions,
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
