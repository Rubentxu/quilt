//! Quilt - AI-first Knowledge Graph
//!
//! Main entry point for the Quilt application.

use anyhow::Result;
use clap::Parser;
use quilt_platform::cli::QuiltCLI;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = QuiltCLI::parse();

    // Run the CLI
    cli.run().await
}
