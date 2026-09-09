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
        events_retention_days: 90,
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
    let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Liveness only — must not advertise maps/agent/store backend posture.
    assert_eq!(v["service"], "altius-api");
    assert!(v.get("maps").is_none());
    assert!(v.get("agent").is_none());
    assert!(v.get("persistence").is_none());
}

#[tokio::test]
async fn ready_fails_closed_without_store() {
    let app = crate::routes::router().with_state(test_state());
    let res = app
        .oneshot(Request::get("/api/v3/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
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

/// Every method the router serves has to survive a browser preflight.
///
/// Allowing only GET and POST let creating a task work while every *edit*
/// failed: the browser asked whether it could send a PUT, got an allow-list
/// without it, and cancelled the request. Nothing reached the API, so nothing
/// appeared in its logs either — the change simply vanished. Task edits,
/// report review and the whole admin section (hubs, teams, roles) went this
/// way. Asserted against the real router so a new verb cannot be routed
/// without also being allowed here.
#[tokio::test]
async fn cors_preflight_allows_every_method_the_router_serves() {
    let state = test_state();
    let mut config = state.config.clone();
    config.cors_origins = vec!["https://web.example".into()];
    let app = crate::routes::router()
        .with_state(state)
        .layer(crate::cors_layer(&config));

    for (method, path) in [
        ("GET", "/api/v3/tasks"),
        ("POST", "/api/v3/task-create"),
        ("PUT", "/api/v3/task/abc"),
        ("PATCH", "/api/v3/reports/driver/2026-01-01"),
        ("DELETE", "/api/v3/task/abc"),
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::options(path)
                    .header("origin", "https://web.example")
                    .header("access-control-request-method", method)
                    .header(
                        "access-control-request-headers",
                        "authorization,content-type",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let allowed = res
            .headers()
            .get("access-control-allow-methods")
            .unwrap_or_else(|| panic!("{method} {path}: preflight returned no allow-methods"))
            .to_str()
            .unwrap()
            .to_owned();
        assert!(
            allowed.split(',').any(|m| m.trim() == method),
            "{method} {path}: browser would cancel this request; allow-methods was {allowed}"
        );
        assert_eq!(
            res.headers()
                .get("access-control-allow-origin")
                .map(|v| v.to_str().unwrap()),
            Some("https://web.example"),
            "{method} {path}: origin not echoed back"
        );
    }
}

/// The allowlist is an allowlist: an origin outside `CORS_ORIGINS` gets no
/// `access-control-allow-origin`, so the browser refuses to hand it the body.
#[tokio::test]
async fn cors_preflight_refuses_an_unlisted_origin() {
    let state = test_state();
    let mut config = state.config.clone();
    config.cors_origins = vec!["https://web.example".into()];
    let app = crate::routes::router()
        .with_state(state)
        .layer(crate::cors_layer(&config));

    let res = app
        .oneshot(
            Request::options("/api/v3/task/abc")
                .header("origin", "https://evil.example")
                .header("access-control-request-method", "PUT")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.headers().get("access-control-allow-origin").is_none(),
        "an unlisted origin was granted access"
    );
}
