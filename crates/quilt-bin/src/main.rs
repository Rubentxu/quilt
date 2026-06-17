//! Quilt - AI-first Knowledge Graph
//!
//! Main entry point for the Quilt application.

use anyhow::Result;
use clap::Parser;
use quilt_application::bootstrap::AppServices;
use quilt_application::services::ref_service::RefService;
use quilt_application::use_cases::*;
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::*;
use quilt_infrastructure::database::sqlite::SqliteAnnotationRepository;
use quilt_platform::cli::QuiltCLI;
use quilt_platform::init;
use quilt_search::SearchService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = QuiltCLI::parse();

    // Resolve the canonical graph root, honoring the --db-path
    // deprecation alias when present. Emits a one-shot warning so
    // users notice the migration.
    let (graph_dir, used_db_path) = cli.resolved_graph_dir();
    if used_db_path {
        eprintln!(
            "warning: --db-path / QUILT_DB_PATH is deprecated; \
             use --graph-dir / QUILT_GRAPH_DIR instead \
             (will be removed in next minor release, see ADR-0030)"
        );
    }

    // Canonical graph bootstrap (ADR-0030) — replaces the legacy
    // direct pool-from-db-path flow.
    let graph_config = init::init_graph(graph_dir)?;

    // Composition root — wire all dependencies here
    let pool = connection::create_pool(&graph_config.db_path).await?;
    connection::run_migrations(&pool).await?;

    let annotation_repo = Arc::new(SqliteAnnotationRepository::new(pool.clone()));
    let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
    let tag_repo = Arc::new(SqliteTagRepository::new(pool.clone()));
    let ref_repo = Arc::new(SqliteRefRepository::new(pool.clone()));
    let ref_service = Arc::new(RefService::new(ref_repo));
    let tour_state_repo = Arc::new(SqliteTourStateRepository::new(pool.clone()));

    let services = AppServices::new(
        Arc::new(AnnotationUseCasesImpl::new(annotation_repo)),
        Arc::new(BlockUseCasesImpl::new(
            block_repo.clone(),
            page_repo.clone(),
            ref_service,
        )),
        Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone())),
        Arc::new(
            SearchUseCasesImpl::new()
                .with_search_service(Arc::new(SearchService::new(Arc::new(pool.clone()))))
                .with_block_repo(block_repo.clone()),
        ),
        Arc::new(ResourceUseCasesImpl::new(block_repo.clone(), page_repo.clone(), tag_repo)),
        Arc::new(TemplateUseCasesImpl::new(page_repo, block_repo)),
        Arc::new(TourStateUseCasesImpl::new(tour_state_repo)),
    );

    // Presentation receives pre-wired services
    cli.run(&services).await
}
