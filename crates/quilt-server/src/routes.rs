//! Router configuration for the HTTP server
//!
//! Sets up the Axum router with all routes and middleware.

use axum::{Router, middleware};
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

    let router = Router::new()
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
        .nest("/api/v1/properties", handlers::properties::routes())
        .nest("/api/v1/schemas", handlers::schemas::routes())
        .nest("/api/v1/references", handlers::references::routes())
        .nest("/api/v1/search", handlers::search::routes())
        .nest("/api/v1/navigate", handlers::navigate::routes())
        .nest("/api/v1/settings", handlers::settings::routes())
        .nest("/api/v1/templates", handlers::templates::routes())
        .nest("/api/v1/query", handlers::query::routes())
        .nest("/api/v1/migration", handlers::migration::routes())
        .nest("/api/v1/user/tour-state", handlers::tour_state::routes())
        .nest("/api/v1/graph", handlers::graph::routes())
        // Frontend serving (catch-all for SPA)
        .route(
            "/",
            axum::routing::get(handlers::frontend::serve_index_html),
        )
        .route(
            "/*path",
            axum::routing::get(handlers::frontend::serve_assets),
        );

    // Layers
    // Order (outermost → innermost):
    //   1. Extension(state)     — state available to all handlers
    //   2. CorsLayer            — handles OPTIONS preflight before auth
    //   3. Auth middleware       — Bearer token check for /api/*
    //   4. TraceLayer           — HTTP tracing (closest to handler)
    router
        .layer(axum::Extension(state))
        .layer(cors)
        .layer(middleware::from_fn(
            crate::middleware::auth::auth_middleware,
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
