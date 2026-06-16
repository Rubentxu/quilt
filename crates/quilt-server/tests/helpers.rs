//! Test helpers for quilt-server tests
//!
//! Provides common setup utilities for integration tests.

use quilt_analysis::agent_room::{AgentLifecycle, AgentRegistry};
use quilt_application::bootstrap::AppServices;
use quilt_application::services::presets::StaticPresetRegistry;
use quilt_application::services::projection::StaticProjectionRegistry;
use quilt_application::services::ref_service::{RefService, RefServiceTrait};
use quilt_application::use_cases::projection_resolver::ProjectionResolver;
use quilt_application::use_cases::{
    BlockUseCases, BlockUseCasesImpl, PageUseCases, PageUseCasesImpl, ResourceUseCases,
    ResourceUseCasesImpl, SearchUseCasesImpl, TemplateUseCases, TemplateUseCasesImpl,
    TourStateUseCases, TourStateUseCasesImpl,
};
use quilt_domain::canonicalization::PresetRegistry;
use quilt_infrastructure::database::sqlite::connection::{DbPool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository, SqlitePropertyRepository, SqliteRefRepository,
    SqliteRelationRepository, SqliteSchemaRepository, SqliteSettingsRepository,
    SqliteTagRepository, SqliteTourStateRepository,
};
use quilt_search::{SearchIndexManager, SearchService};
use quilt_server::state::RepositoryBundle;
use std::sync::Arc;

/// Build a minimal AppState for testing.
///
/// This creates an in-memory database, initializes all repositories,
/// and bundles them into AppServices.
pub async fn build_test_app_state(pool: DbPool) -> quilt_server::state::AppState {
    let (state, _, _, _, _, _, _, _, _, _) = build_test_app_state_with_repos(pool).await;
    state
}

/// Build a minimal AppState for testing, returning repositories for seeding.
///
/// This creates an in-memory database, initializes all repositories,
/// and bundles them into AppServices. The concrete repositories are returned
/// for test functions that need to seed data before the app is used.
pub async fn build_test_app_state_with_repos(
    pool: DbPool,
) -> (
    quilt_server::state::AppState,
    Arc<SqliteBlockRepository>,
    Arc<SqlitePageRepository>,
    Arc<SqliteRefRepository>,
    Arc<SqliteTagRepository>,
    Arc<SqliteSettingsRepository>,
    Arc<SqliteRelationRepository>,
    Arc<SqliteSchemaRepository>,
    Arc<SqlitePropertyRepository>,
    Arc<SqliteTourStateRepository>,
) {
    // Run migrations first
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));
    let search_service: Arc<SearchService> = Arc::new(SearchService::new(Arc::new(pool.clone())));

    // Initialize bidirectional reference service
    let ref_repo: Arc<SqliteRefRepository> = Arc::new(SqliteRefRepository::new(pool.clone()));
    let mut ref_service = RefService::new(ref_repo.clone());
    // Note: rebuild_from_repo might fail on empty DB, but we ignore for testing
    let _ = ref_service.rebuild_from_repo();
    let ref_service: Arc<dyn RefServiceTrait> = Arc::new(ref_service);

    // Build repositories
    let block_repo: Arc<SqliteBlockRepository> = Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo: Arc<SqlitePageRepository> = Arc::new(SqlitePageRepository::new(pool.clone()));
    let tag_repo: Arc<SqliteTagRepository> = Arc::new(SqliteTagRepository::new(pool.clone()));
    let tour_state_repo: Arc<SqliteTourStateRepository> =
        Arc::new(SqliteTourStateRepository::new(pool.clone()));
    let settings_repo: Arc<SqliteSettingsRepository> =
        Arc::new(SqliteSettingsRepository::new(pool.clone()));
    let relation_repo: Arc<SqliteRelationRepository> =
        Arc::new(SqliteRelationRepository::new(pool.clone()));
    let schema_repo: Arc<SqliteSchemaRepository> =
        Arc::new(SqliteSchemaRepository::new(pool.clone()));
    let property_repo: Arc<SqlitePropertyRepository> =
        Arc::new(SqlitePropertyRepository::new(pool.clone()));

    // Build use cases
    let block_use_cases: Arc<dyn BlockUseCases> = Arc::new(BlockUseCasesImpl::new(
        block_repo.clone(),
        page_repo.clone(),
        ref_service.clone(),
    ));
    let page_use_cases: Arc<dyn PageUseCases> =
        Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone()));
    let resource_use_cases: Arc<dyn ResourceUseCases> = Arc::new(ResourceUseCasesImpl::new(
        block_repo.clone(),
        page_repo.clone(),
        tag_repo.clone(),
    ));
    let template_use_cases: Arc<dyn TemplateUseCases> = Arc::new(TemplateUseCasesImpl::new(
        page_repo.clone(),
        block_repo.clone(),
    ));
    let tour_state_use_cases: Arc<dyn TourStateUseCases> =
        Arc::new(TourStateUseCasesImpl::new(tour_state_repo.clone()));

    let search_use_cases = Arc::new(
        SearchUseCasesImpl::new()
            .with_search_service(search_service.clone())
            .with_block_repo(block_repo.clone()),
    );

    let services = AppServices::new(
        block_use_cases,
        page_use_cases,
        search_use_cases,
        resource_use_cases,
        template_use_cases,
        tour_state_use_cases,
    );

    // Bundle all repositories
    let repos = RepositoryBundle::new(
        block_repo.clone(),
        page_repo.clone(),
        ref_repo.clone(),
        settings_repo.clone(),
        tag_repo.clone(),
        relation_repo.clone(),
        schema_repo.clone(),
        property_repo.clone(),
        tour_state_repo.clone(),
    );

    let state = quilt_server::state::AppState::new_with_repos(
        repos,
        search_service,
        search_index,
        ref_service,
        Arc::new(services),
        Arc::new(ProjectionResolver::new(StaticProjectionRegistry::v1())),
        Arc::new(StaticPresetRegistry::v1()) as Arc<dyn PresetRegistry>,
    );

    (
        state,
        block_repo,
        page_repo,
        ref_repo,
        tag_repo,
        settings_repo,
        relation_repo,
        schema_repo,
        property_repo,
        tour_state_repo,
    )
}

/// Build an AppState with the Agent Room wired in (CG-5).
/// Returns the state, the lifecycle, and the registry for
/// tests that want to assert on internal state.
pub async fn build_test_app_state_with_agents(
    pool: DbPool,
) -> (
    quilt_server::state::AppState,
    Arc<AgentLifecycle>,
    Arc<AgentRegistry>,
) {
    let (state, _block_repo, _page_repo, _, _, _, _, _, _, _) =
        build_test_app_state_with_repos(pool).await;
    let lifecycle = Arc::new(AgentLifecycle::new(
        state.repos.block.clone(),
        state.repos.page.clone(),
    ));
    let registry = Arc::new(AgentRegistry::with_defaults());

    // Spawn the worker so the HTTP handlers see real
    // executor-driven state transitions. The handle is
    // intentionally dropped — in production the server
    // holds it for clean shutdown; tests just need the
    // worker running.
    let queue = quilt_analysis::agent_room::AgentQueue::new(
        (*lifecycle).clone(),
        registry.clone(),
    );
    let _ = quilt_analysis::agent_room::queue::spawn_worker(queue);

    let state = quilt_server::state::AppState::new_with_repos_and_agents(
        state.repos.clone(),
        state.search_service.clone(),
        state.search_index.clone(),
        state.ref_service.clone(),
        state.services.clone(),
        state.projection_resolver.clone(),
        state.preset_registry.clone(),
        Some(lifecycle.clone()),
        Some(registry.clone()),
    );
    (state, lifecycle, registry)
}
