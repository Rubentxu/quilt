//! Metrics HTTP handler
//!
//! Exposes Prometheus metrics at /metrics when QUILT_METRICS=true.

use axum::{body::Body, http::StatusCode, response::IntoResponse};

// Global Prometheus handle - initialized once at startup
static METRICS_HANDLE: std::sync::OnceLock<metrics_exporter_prometheus::PrometheusHandle> =
    std::sync::OnceLock::new();

/// Initialize Prometheus metrics exporter.
/// Should be called once at startup.
pub fn init_metrics() -> bool {
    if std::env::var("QUILT_METRICS").unwrap_or_default() != "true" {
        tracing::info!("Metrics: disabled (set QUILT_METRICS=true to enable)");
        return false;
    }

    tracing::info!("Metrics: enabled");

    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    match builder.install_recorder() {
        Ok(handle) => {
            if METRICS_HANDLE.set(handle).is_err() {
                tracing::warn!("Metrics handle already initialized");
            }
            tracing::info!("Metrics endpoint available at /metrics");
            true
        }
        Err(e) => {
            tracing::error!("Failed to install Prometheus metrics: {}", e);
            false
        }
    }
}

/// GET /metrics
///
/// Returns Prometheus-formatted metrics.
pub async fn metrics_handler() -> impl IntoResponse {
    // Check if metrics are enabled
    if METRICS_HANDLE.get().is_none() {
        return (
            StatusCode::NOT_FOUND,
            [("Content-Type", "text/plain")],
            Body::from("Metrics not enabled. Set QUILT_METRICS=true"),
        )
            .into_response();
    }

    // Render metrics using the installed handle
    let metrics = METRICS_HANDLE.get().unwrap().render();

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        Body::from(metrics),
    )
        .into_response()
}
