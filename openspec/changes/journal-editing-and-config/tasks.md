# Tasks: journal-editing-and-config

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 600-900 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Delivery strategy | ask-on-risk |
| Decision needed before apply | Yes |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | PR | Base |
|------|------|-----|------|
| 1 | Infrastructure: SqliteSettingsRepository + Settings REST API | PR 1 | main |
| 2 | Foundation: BlockDto consolidation + JournalDay::to_formatted | PR 2 | PR 1 |
| 3 | Wiring: Journal unification + on_save + UI settings | PR 3 | PR 2 |

## Phase 1: Foundation

- [x] 1.1 `crates/quilt-domain/src/value_objects/journal_day.rs` — add `to_formatted(&str) -> String` using `NaiveDate::format()`
- [x] 1.2 `crates/quilt-application/src/use_cases/dtos.rs` — create file with canonical `BlockDto` (id, page_name, content, level: u8, children, created_at, updated_at)
- [x] 1.3 `crates/quilt-application/src/use_cases/mod.rs` — export `BlockDto` from dtor
- [x] 1.4 `crates/quilt-application/src/lib.rs` — re-export `BlockDto` publicly

## Phase 2: Infrastructure

- [x] 2.1 `crates/quilt-infrastructure/src/database/sqlite/repositories.rs` — implement `SqliteSettingsRepository` (get/update_user_settings using config table)
- [x] 2.2 `crates/quilt-domain/src/entities/user_settings.rs` — add `"%d-%m-%Y"` to `common_date_formats()` for Spain format
- [x] 2.3 `crates/quilt-server/src/handlers/settings.rs` — create file with `get_settings` and `update_settings` handlers
- [x] 2.4 `crates/quilt-server/src/handlers/mod.rs` — add `pub mod settings;`
- [x] 2.5 `crates/quilt-server/src/routes.rs` — add `"/settings"` route with GET and PUT
- [x] 2.6 `crates/quilt-server/src/state.rs` — add `settings_repo: Arc<dyn SettingsRepository>` to AppState and initialize in `new()`
- [x] 2.7 `crates/quilt-server/src/handlers/blocks.rs` — remove local BlockDto, import from application layer (deferred to PR 2)

## Phase 3: Wiring & Frontend

- [x] 3.1 `crates/quilt-ui/src/bridge.rs` — add `UserSettingsDto`, `SettingsState`, `get_settings()` and `update_settings()` functions
- [x] 3.2 `crates/quilt-ui/src/bridge.rs` — add `SettingsState` (reactive wrapper) with `load()` and `update()` methods (moved from state.rs due to pre-existing compilation issues in state.rs)
- [x] 3.3 `crates/quilt-ui/src/components/block.rs` — modify `on_save` to call `bridge::update_block(id, content)` via `spawn_local`
- [x] 3.4 `crates/quilt-ui/src/pages/page.rs` — add journal date navigation header (from JournalView) and journal-aware routing
- [x] 3.5 `crates/quilt-ui/src/pages/journal.rs` — replace JournalView with redirect or remove entirely
- [x] 3.6 `crates/quilt-ui/src/app.rs` — change route `/journal/:date` from JournalView to PageView
- [x] 3.7 `crates/quilt-ui/src/components/sidebar.rs` — update calendar to use configured `journal_format` for display tooltip; route paths stay ISO for server compatibility

## Phase 4: Testing

- [x] 4.1 Unit: `journal_day.rs` — add tests for `to_formatted` with "%Y-%m-%d", "%d-%m-%Y", "%B %d, %Y"
- [x] 4.2 Integration: `SqliteSettingsRepository` — test get/update cycle (test_settings_get_default_when_empty + test_settings_update_cycle)
- [ ] 4.3 Integration: `GET /api/v1/settings` — blocked: needs http test framework (axum-test not in dev deps)
- [ ] 4.4 Integration: `PUT /api/v1/settings` — blocked: needs http test framework (axum-test not in dev deps)
- [ ] 4.5 E2E: Journal page — manual Playwright (run separately)
- [ ] 4.6 E2E: Journal title — manual Playwright (run separately)

## Phase 5: Cleanup

- [x] 5.1 Verify `cargo test` — 146 tests pass (25 domain + 73 application + 48 infrastructure)
- [ ] 5.2 Verify `cargo clippy` — pre-existing error in quilt-domain/src/references/ref_type.rs (unrelated)
- [x] 5.3 Verify build with `cargo build --all` — builds clean (7 pre-existing warnings in quilt-server)
