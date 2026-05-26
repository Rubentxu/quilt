//! Quilt - AI-first Knowledge Graph
//!
//! Main entry point for the Quilt application.

use anyhow::Result;
use clap::Parser;
use quilt_application::bootstrap::AppServices;
use quilt_application::use_cases::*;
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::*;
use quilt_platform::cli::QuiltCLI;
use quilt_search::SearchService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = QuiltCLI::parse();

    // Composition root — wire all dependencies here
    let pool = connection::create_pool(&cli.db_path).await?;
    connection::run_migrations(&pool).await?;

    let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
    let tag_repo = Arc::new(SqliteTagRepository::new(pool.clone()));

    let services = AppServices::new(
        Arc::new(BlockUseCasesImpl::new(
            block_repo.clone(),
            page_repo.clone(),
        )),
        Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone())),
        Arc::new(
            SearchUseCasesImpl::new()
                .with_search_service(Arc::new(SearchService::new(Arc::new(pool.clone()))))
                .with_block_repo(block_repo.clone()),
        ),
        Arc::new(ResourceUseCasesImpl::new(block_repo, page_repo, tag_repo)),
    );

    // Presentation receives pre-wired services
    cli.run(&services).await
}
