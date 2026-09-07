mod agent;
mod auth;
mod config;
mod error;
mod maps;
mod routes;
mod store;

#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::http::{header::HeaderValue, Method};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::auth::Jwks;
use crate::config::Config;

pub struct AppState {
    pub config: Config,
    pub jwks: Jwks,
    pub http: reqwest::Client,
    pub store: Option<store::Store>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;
    let jwks = Jwks::new(config.keycloak.clone());

    let typedb_addr = std::env::var("TYPEDB_ADDRESS")
        .context("TYPEDB_ADDRESS required for production mode")?;
    let typedb_user = std::env::var("TYPEDB_USERNAME")
        .context("TYPEDB_USERNAME required for production mode")?;
    let typedb_pass = std::env::var("TYPEDB_PASSWORD")
        .context("TYPEDB_PASSWORD required for production mode")?;
    let driver = altius_schema::connect(&typedb_addr, &typedb_user, &typedb_pass).await?;
    altius_schema::migrate(&driver, &config.typedb_database).await?;
    let store = store::Store::new(driver, config.typedb_database.clone());
    store
        .ensure_default_org_hub(&config)
        .await
        .context("provision default organization and hub")?;
    store
        .link_admin_user(&config)
        .await
        .context("link default admin user")?;

    let cors = cors_layer(&config);

    let state = Arc::new(AppState {
        config,
        jwks,
        http: reqwest::Client::new(),
        store: Some(store),
    });

    let app = Router::new()
        .merge(routes::router())
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        // NOTE: per-IP rate limiting belongs at the edge/LB, not here.
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'none'"),
        ))
        .layer(cors)
        .with_state(state);

    let addr = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    tracing::info!(%addr, "altius-api listening");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// CORS allowlist from `CORS_ORIGINS` (comma-separated). Empty → deny all
/// cross-origin browser calls (API is server-to-server / native clients).
fn cors_layer(config: &Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
        .max_age(Duration::from_secs(600));
    if origins.is_empty() {
        layer.allow_origin(AllowOrigin::list(Vec::<HeaderValue>::new()))
    } else {
        layer.allow_origin(AllowOrigin::list(origins))
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
