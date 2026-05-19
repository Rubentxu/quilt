//! Quilt HTTP Server Binary
//!
//! Entry point for the HTTP server that provides REST API and WebSocket endpoints.

use std::path::PathBuf;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use quilt_platform::init::HttpServerInit;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Read configuration from environment variables
    let port = std::env::var("QUILT_HTTP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .context("Invalid QUILT_HTTP_PORT")?;

    let host = std::env::var("QUILT_HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let vault_path = std::env::var("QUILT_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    tracing::info!("Starting Quilt HTTP server");
    tracing::info!("Configuration: host={}, port={}, vault={:?}", host, port, vault_path);

    // Initialize shared state using platform initialization
    let init = HttpServerInit::new(vault_path.clone()).await?;

    tracing::info!("Vault ready at {:?}", init.vault_config.vault_path);
    tracing::info!("Database pool created");

    // Run the HTTP server
    quilt_http::run_http_server(
        init.pool,
        init.vault_config.vault_path,
        Some(init.mcp_server),
        &host,
        port,
    )
    .await
}

// Add context helper for anyhow
trait Context<T> {
    fn context(self, msg: &str) -> anyhow::Result<T>
    where
        Self: Sized;
}

impl<T> Context<T> for Option<T> {
    fn context(self, _msg: &str) -> anyhow::Result<T>
    where
        Self: Sized,
    {
        self.ok_or_else(|| anyhow::anyhow!("Context: {}", _msg))
    }
}

impl<T, E: std::fmt::Display> Context<T> for Result<T, E> {
    fn context(self, msg: &str) -> anyhow::Result<T>
    where
        Self: Sized,
    {
        self.map_err(|e| anyhow::anyhow!("{}: {}", msg, e))
    }
}