mod admin;
mod agent;
mod auth;
mod config;
mod error;
mod maps;
mod mceasy;
mod mceasy_worker;
mod notify;
mod routes;
mod store;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod pg_it;

#[cfg(test)]
mod typedb_it;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::Router;
use axum::http::{Method, header::HeaderValue};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::auth::Jwks;
use crate::config::Config;
use crate::mceasy::MceasyClient;

mod fcm;
mod metrics;

pub struct AppState {
    pub config: Config,
    pub jwks: Jwks,
    pub http: reqwest::Client,
    pub store: Option<store::Store>,
    pub mceasy_client: Option<MceasyClient>,
    pub fcm_client: Option<fcm::FcmClient>,
    pub metrics: Arc<metrics::Metrics>,
}

/// Probe the local readiness endpoint and exit 0/1.
///
/// The runtime image is `debian-slim`: no curl, no wget, and `/bin/sh` is dash,
/// which has no `/dev/tcp`. Rather than install a package just to answer
/// "are you ready", the binary probes itself.
async fn health_check() -> std::process::ExitCode {
    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let port = bind.rsplit(':').next().unwrap_or("8080");
    let url = format!("http://127.0.0.1:{port}/api/v3/ready");
    let ok = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map(|c| c.get(&url).send())
        .ok();
    match ok {
        Some(fut) => match fut.await {
            Ok(res) if res.status().is_success() => std::process::ExitCode::SUCCESS,
            Ok(res) => {
                eprintln!("not ready: {}", res.status());
                std::process::ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("not ready: {e}");
                std::process::ExitCode::FAILURE
            }
        },
        None => std::process::ExitCode::FAILURE,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<std::process::ExitCode> {
    if std::env::args().any(|a| a == "--health-check") {
        return Ok(health_check().await);
    }

    // JSON in containers so a log shipper can parse fields; pretty text when a
    // developer is reading the terminal. LOG_FORMAT=text overrides.
    let filter = tracing_subscriber::EnvFilter::from_default_env();
    if std::env::var("LOG_FORMAT").as_deref() == Ok("text") {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_env_filter(filter)
            .init();
    }

    let config = Config::from_env()?;
    let jwks = Jwks::new(config.keycloak.clone());

    let store = match config.store_backend {
        crate::config::StoreBackend::Postgres => {
            let pool = crate::store::pg::connect(&config.database_url)
                .await
                .context("connect postgres")?;
            crate::store::pg::migrate(&pool)
                .await
                .context("migrate postgres")?;
            store::Store::Postgres(store::PgStore::new(pool))
        }
        crate::config::StoreBackend::Typedb => {
            let typedb_addr = std::env::var("TYPEDB_ADDRESS")
                .context("TYPEDB_ADDRESS required when STORE_BACKEND=typedb")?;
            let typedb_user = std::env::var("TYPEDB_USERNAME")
                .context("TYPEDB_USERNAME required when STORE_BACKEND=typedb")?;
            let typedb_pass = std::env::var("TYPEDB_PASSWORD")
                .context("TYPEDB_PASSWORD required when STORE_BACKEND=typedb")?;
            let driver = altius_schema::connect(&typedb_addr, &typedb_user, &typedb_pass).await?;
            altius_schema::migrate(&driver, &config.typedb_database).await?;
            store::Store::Typedb(store::TypedbStore::new(
                driver,
                config.typedb_database.clone(),
            ))
        }
    };
    store
        .ensure_default_org_hub(&config)
        .await
        .context("provision default organization and hub")?;
    store
        .link_admin_user(&config)
        .await
        .context("link default admin user")?;

    let cors = cors_layer(&config);

    let mceasy_client = config.mceasy.as_ref().map(|cfg| {
        MceasyClient::new(
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .expect("build mceasy http client"),
            Arc::new(cfg.clone()),
        )
    });

    let fcm_client = if config.fcm_project_id.is_some() || config.fcm_credentials_path.is_some() {
        match fcm::FcmClient::from_config(&config).await {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::error!(error = %e, "fcm client failed to initialize");
                None
            }
        }
    } else {
        None
    };

    let metrics =
        Arc::new(crate::metrics::Metrics::new().expect("build prometheus metrics registry"));

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    if let (Some(client), Some(store)) = (mceasy_client.clone(), Some(store.clone())) {
        let worker = crate::mceasy_worker::MceasyWorker {
            client,
            store: Arc::new(store),
        };
        tokio::spawn(async move {
            worker.run(shutdown_rx).await;
        });
    }

    let state = Arc::new(AppState {
        config,
        jwks,
        // Every outbound call — Keycloak, Google Maps, OpenRouter, the SMS
        // gateway — shares this client. Without an explicit timeout a hung
        // upstream holds a request handler open indefinitely.
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("build http client"),
        store: Some(store),
        mceasy_client,
        fcm_client,
        metrics,
    });

    // Hold the shutdown sender in the main task. When the serve future returns,
    // the sender is dropped and the watch channel fires, stopping the worker.
    let _shutdown_guard = shutdown_tx;

    let app = Router::new()
        .merge(routes::router())
        .merge(metrics::router())
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
    Ok(std::process::ExitCode::SUCCESS)
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
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
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
