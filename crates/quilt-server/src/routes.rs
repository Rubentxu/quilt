//! Router configuration for the HTTP server
//!
//! Sets up the Axum router with all routes and middleware.

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::handlers;
use crate::state::AppState;

/// Create the main application router
pub fn create_app(state: AppState) -> Router {
    let cors = if std::env::var("QUILT_CORS").unwrap_or_default() == "true" {
        tracing::info!("CORS: permissive mode enabled");
        CorsLayer::permissive()
    } else {
        tracing::info!("CORS: disabled");
        CorsLayer::new()
    };

    Router::new()
        // Health check
        .route(
            "/health",
            axum::routing::get(handlers::health::health_check),
        )
        // Metrics endpoint (only works if QUILT_METRICS=true)
        .route(
            "/metrics",
            axum::routing::get(handlers::metrics::metrics_handler),
        )
        // WebSocket endpoint
        .route(
            "/ws",
            axum::routing::get(handlers::websocket::websocket_handler),
        )
        // API v1 routes
        .nest("/api/v1/blocks", handlers::blocks::routes())
        .nest("/api/v1/pages", handlers::pages::routes())
        .nest("/api/v1/search", handlers::search::routes())
        .nest("/api/v1/cognitive", handlers::cognitive::routes())
        .nest("/api/v1/ai-config", handlers::ai_config::routes())
        .nest("/api/v1/navigate", handlers::navigate::routes())
        // Frontend serving (catch-all for SPA)
        .route(
            "/",
            axum::routing::get(handlers::frontend::serve_index_html),
        )
        .route(
            "/*path",
            axum::routing::get(handlers::frontend::serve_assets),
        )
        // Layers
        .layer(axum::Extension(state))
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
