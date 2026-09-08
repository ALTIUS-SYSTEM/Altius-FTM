//! Prometheus metrics endpoint and application counters.

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Router, http::StatusCode};
use prometheus::{
    Counter, Encoder, Histogram, HistogramOpts, Registry, TextEncoder, exponential_buckets,
};

use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Global metrics registry.
#[derive(Clone)]
pub struct Metrics {
    pub registry: Arc<Registry>,
    pub http_requests_total: Counter,
    pub http_request_duration: Histogram,
    pub fcm_send_total: Counter,
    pub mceasy_poll_total: Counter,
    pub mceasy_poll_errors: Counter,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Arc::new(Registry::new());

        let http_requests_total = Counter::with_opts(prometheus::Opts::new(
            "altius_http_requests_total",
            "Total HTTP requests received",
        ))?;
        registry.register(Box::new(http_requests_total.clone()))?;

        let http_request_duration = Histogram::with_opts(
            HistogramOpts::new(
                "altius_http_request_duration_seconds",
                "HTTP request duration",
            )
            .buckets(exponential_buckets(0.001, 2.0, 15).unwrap_or_default()),
        )?;
        registry.register(Box::new(http_request_duration.clone()))?;

        let fcm_send_total = Counter::with_opts(prometheus::Opts::new(
            "altius_fcm_send_total",
            "Total FCM push notifications sent",
        ))?;
        registry.register(Box::new(fcm_send_total.clone()))?;

        let mceasy_poll_total = Counter::with_opts(prometheus::Opts::new(
            "altius_mceasy_poll_total",
            "Total McEasy polls",
        ))?;
        registry.register(Box::new(mceasy_poll_total.clone()))?;

        let mceasy_poll_errors = Counter::with_opts(prometheus::Opts::new(
            "altius_mceasy_poll_errors_total",
            "Total McEasy poll failures",
        ))?;
        registry.register(Box::new(mceasy_poll_errors.clone()))?;

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            fcm_send_total,
            mceasy_poll_total,
            mceasy_poll_errors,
        })
    }
}

async fn metrics_handler(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = s.metrics.registry.gather();
    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            buffer,
        ),
        Err(e) => {
            tracing::error!(error = %e, "metrics encode failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(axum::http::header::CONTENT_TYPE, "text/plain")],
                format!("metrics encode failed: {e}").into_bytes(),
            )
        }
    }
}
