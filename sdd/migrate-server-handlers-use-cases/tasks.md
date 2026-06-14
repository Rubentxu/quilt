# Task Breakdown: server-handlers-use-case-layer

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Medium

## Phase 1: Foundation (prerequisites)

- [ ] Task 1.1: Expand `AppServices` in `crates/quilt-application/src/bootstrap.rs` with `template: Arc<dyn TemplateUseCases>`, `tour_state: Arc<dyn TourStateUseCases>`, `annotation: Arc<dyn AnnotationUseCases>`.
- [ ] Task 1.2: Add `services: AppServices` to `AppState` and `impl FromRef<AppState> for AppServices` in `crates/quilt-server/src/state.rs`.
- [ ] Task 1.3: Wire `AppServices` in `crates/quilt-server/src/main.rs` (mirror MCP pattern: build repos once, wrap in use cases, pass to `AppState::new_with_services`).
- [ ] Task 1.4: Add `get_by_id`, `update`, `delete_with_children_check`, `get_properties`, `set_property`, `delete_property` to `BlockUseCases` and impl in `crates/quilt-application/src/use_cases/block.rs`.
- [ ] Task 1.5: Add `get_by_name`, `search_by_name_or_title`, `create_journal` to `PageUseCases` and impl in `crates/quilt-application/src/use_cases/page.rs`.
- [ ] Task 1.6: Add `search_blocks` to `SearchUseCases` and impl in `crates/quilt-application/src/use_cases/search.rs`.
- [ ] Task 1.7: Implement new use-case methods with unit tests using `InMemoryBlockRepo` / `InMemoryPageRepo` in `crates/quilt-application/src/use_cases/`.

## Phase 2: Batch 1 — Core handlers (blocks.rs, pages.rs)

- [ ] Task 2.1: Migrate `crates/quilt-server/src/handlers/blocks.rs` to extract `Extension<AppServices>`; replace `SqliteBlockRepository` / `SqlitePageRepository` construction with `services.block` and `services.search` calls. Keep `ref_service` as a separate `Extension<Arc<RwLock<RefService>>>`.
- [ ] Task 2.2: Migrate `crates/quilt-server/src/handlers/pages.rs` to extract `Extension<AppServices>`; replace repo construction with `services.page` calls. Keep `settings_repo` and `ref_service` as separate extensions.
- [ ] Task 2.3: Add handler-level tests for migrated blocks/pages endpoints in `crates/quilt-server/src/handlers/`.

## Phase 3: Batch 2 — Graph/query handlers

- [ ] Task 3.1: Migrate `crates/quilt-server/src/handlers/graph.rs` — use `AppServices` for block/page reads, keep `ref_service` as a separate extension for BFS forward-ref lookups.
- [ ] Task 3.2: Migrate `crates/quilt-server/src/handlers/references.rs` — use `AppServices` for block/page lookups; keep `ref_service` for ref mutations.
- [ ] Task 3.3: Migrate `crates/quilt-server/src/handlers/query.rs` — use `services.search.query()` instead of direct `QueryExecutorService`.
- [ ] Task 3.4: Migrate `crates/quilt-server/src/handlers/migration.rs` — use `services.block` and `services.page` for repo access.

## Phase 4: Batch 3 — Remaining handlers

- [ ] Task 4.1: Migrate `crates/quilt-server/src/handlers/properties.rs` — use `AppServices` for block lookups; keep `PropertyService` for property-specific logic.
- [ ] Task 4.2: Migrate `crates/quilt-server/src/handlers/templates.rs` — replace per-request `build_use_cases` with `AppServices.template`.
- [ ] Task 4.3: Migrate `crates/quilt-server/src/handlers/search.rs` — use `services.search.search()` instead of direct `SearchService`.
- [ ] Task 4.4: Migrate `crates/quilt-server/src/handlers/tour_state.rs` — replace per-request `build_use_cases` with `AppServices.tour_state`.

## Phase 5: Batch 4 — Cleanup

- [ ] Task 5.1: Remove `pool` field from `AppState` in `crates/quilt-server/src/state.rs` and delete `impl FromRef<AppState> for DbPool`.
- [ ] Task 5.2: Delete `crates/quilt-application/src/commands.rs` (superseded by use-case traits).
- [ ] Task 5.3: Collapse `BlockDto` `From` impls in `crates/quilt-server/src/handlers/blocks.rs` into a single `From<Block>` with an optional `page_name` helper.
- [ ] Task 5.4: Update all tests to construct `AppState` via `new_with_services` instead of `new`.
- [ ] Task 5.5: Run `cargo test` and `just test-e2e` for final integration verification.
