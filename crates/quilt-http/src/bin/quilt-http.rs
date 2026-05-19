//! Quilt HTTP Server Binary
//!
//! Entry point for the HTTP server CLI.

use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use quilt_platform::init::HttpServerInit;

/// CLI arguments for the HTTP server
#[derive(Parser)]
#[command(name = "quilt-http")]
#[command(about = "Quilt HTTP Server - REST API and WebSocket endpoints", long_about = None)]
struct Cli {
    /// Vault directory path
    #[arg(long, env = "QUILT_VAULT_PATH")]
    vault_path: Option<PathBuf>,

    /// HTTP server port
    #[arg(long, env = "QUILT_HTTP_PORT", default_value = "8080")]
    port: u16,

    /// HTTP server host
    #[arg(long, env = "QUILT_HTTP_HOST", default_value = "127.0.0.1")]
    host: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    let vault_path = cli.vault_path.unwrap_or_else(|| PathBuf::from("."));

    tracing::info!("Starting Quilt HTTP server");
    tracing::info!("Configuration: host={}, port={}, vault={:?}", cli.host, cli.port, vault_path);

    // Initialize shared state using platform initialization
    let init = HttpServerInit::new(vault_path.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize: {}", e))?;

    tracing::info!("Vault ready at {:?}", init.vault_config.vault_path);
    tracing::info!("Database pool created");

    // Run the HTTP server
    quilt_http::run_http_server(
        init.pool,
        init.vault_config.vault_path,
        Some(init.mcp_server),
        &cli.host,
        cli.port,
    )
    .await
}