## Exploration: Migrate server handlers to use-case layer

### Current State

The quilt-server crate (`crates/quilt-server/src/handlers/`) has **every handler** directly importing and constructing `Sqlite*Repository` types from the infrastructure layer. They extract `DbPool` via `Extension(pool)` and build repos inline:

```rust
// Antipattern — in every handler function:
let block_repo = SqliteBlockRepository::new(pool.clone());
let page_repo = SqlitePageRepository::new(pool.clone());
```

This bypasses the entire application layer (`quilt-application`), which has mature use-case traits and implementations. The MCP layer (`quilt-mcp`) already uses the correct pattern: handlers take `Arc<dyn BlockUseCases>` as a dependency, and the server bootstrap wires repos into use-case impls once at startup.

`AppState.pool` is already marked **DEPRECATED** in `state.rs` with the comment: _"Pool is an infrastructure detail. Handlers should not construct repositories directly. Will be removed in Phase 2."_

### Affected Areas

**Server handlers that construct repos directly (7 files):**

| File | Repos Used | Operations |
|------|------------|------------|
| `handlers/blocks.rs` | `SqliteBlockRepository`, `SqlitePageRepository` | create, read, update, delete, backlinks, link, properties, author queries |
| `handlers/pages.rs` | `SqliteBlockRepository`, `SqlitePageRepository`, `SqliteRefRepository` | CRUD, unlinked refs, rename, delete, properties |
| `handlers/graph.rs` | `SqliteBlockRepository`, `SqlitePageRepository` | BFS subgraph traversal, lens queries |
| `handlers/navigate.rs` | `SqlitePageRepository` | page lookup for navigation resolution |
| `handlers/references.rs` | `SqliteBlockRepository`, `SqlitePageRepository`, `SqliteRefRepository` | ref creation, lookup, deletion |
| `handlers/query.rs` | `SqliteBlockRepository` | DSL query execution |
| `handlers/migration.rs` | `SqliteBlockRepository`, `SqlitePageRepository` | graph import/export |
| `handlers/properties.rs` | `SqliteBlockRepository` | block property CRUD |

**Server handlers that already use (or nearly use) the correct pattern:**

| File | Current Pattern | Notes |
|------|----------------|-------|
| `handlers/templates.rs` | Builds `TemplateUseCasesImpl` per-request via `build_use_cases(&pool)` | Correct trait usage but recreates impl on every request (should be cached at startup) |
| `handlers/search.rs` | Uses `SearchService` directly from `quilt_search` | Bypasses `SearchUseCases` trait; should use `SearchUseCases` instead |
| `handlers/settings.rs` | Uses `SqliteSettingsRepository` directly | No `SettingsUseCases` trait exists yet — needs one or uses `SettingsCommand` |

**Correct pattern reference — MCP handlers (`crates/quilt-mcp/src/handlers/`):**

| File | Injected Dependency |
|------|-------------------|
| `block.rs` | `Arc<dyn BlockUseCases>` |
| `page.rs` | `Arc<dyn PageUseCases>` |
| `graph.rs` | `Arc<dyn BlockUseCases>` |
| `resource.rs` | `Arc<dyn ResourceUseCases>` |
| `query.rs` | `Arc<dyn SearchUseCases>` |
| `retrieval.rs` | `Arc<dyn SearchUseCases>` |
| `temporal.rs` | `Arc<dyn SearchUseCases>` |
| `template.rs` | `Arc<dyn TemplateUseCases>` |
| `cognitive.rs` | `Arc<dyn BlockUseCases>` |

### Use-Case Traits Available

All in `crates/quilt-application/src/use_cases/`:

| Trait | File | Methods | Implementation | Generic Params |
|-------|------|---------|----------------|----------------|
| `BlockUseCases` | `block.rs` | `create_with_page`, `create_task`, `delete`, `link`, `get_tree`, `get_backlinks`, `list_by_property` | `BlockUseCasesImpl<BR, PR>` | `BR: BlockRepository`, `PR: PageRepository` |
| `PageUseCases` | `page.rs` | `create`, `list`, `get_blocks`, `get_or_create_journal`, `update_properties` | `PageUseCasesImpl<PR, BR>` | `PR: PageRepository`, `BR: BlockRepository` |
| `SearchUseCases` | `search.rs` | `search`, `query`, `resolve_by_name` | `SearchUseCasesImpl` (builder) | None (uses `Arc<dyn SearchServiceTrait>`) |
| `ResourceUseCases` | `resource.rs` | `graph_snapshot`, `list_pages`, `list_journals`, `list_tags` | `ResourceUseCasesImpl<BR, PR, TR>` | `BR: BlockRepository`, `PR: PageRepository`, `TR: TagRepository` |
| `TemplateUseCases` | `template.rs` | `list_templates`, `get_template_schema` | `TemplateUseCasesImpl<PR, BR>` | `PR: PageRepository`, `BR: BlockRepository` |
| `TourStateUseCases` | `tour_state.rs` | tour state CRUD | `TourStateUseCasesImpl<R>` | `R: TourStateRepository` |

**All six traits have implementations.** All are object-safe (`Send + Sync`), all use `#[async_trait]`.

### AppServices vs AppState — the Gap

**`AppServices`** (bootstrap.rs) is the composition root — bundles all 4 core use cases:
```rust
pub struct AppServices {
    pub block: Arc<dyn BlockUseCases>,
    pub page: Arc<dyn PageUseCases>,
    pub search: Arc<dyn SearchUseCases>,
    pub resource: Arc<dyn ResourceUseCases>,
}
```

**`AppState`** (state.rs) currently holds raw infrastructure:
```rust
pub struct AppState {
    pub pool: DbPool,                          // DEPRECATED
    pub settings_repo: SqliteSettingsRepository,
    pub search_index: Arc<SearchIndexManager>,
    pub navigation_channel: NavigationChannel,
    pub last_opened_graph: Arc<RwLock<Option<String>>>,
    pub ref_service: Arc<RwLock<RefService>>,
    pub event_bus: Arc<EventBus>,
}
```

The gap is clear: AppState has no use-case layer at all. Handlers go directly from `pool` → `SqliteBlockRepository`. The `state.rs` comments even say: _"Future phases will add `Arc<dyn BlockUseCases>`, `Arc<dyn PageUseCases>`, etc. as individual fields here."_

### What about `BlockCommand`?

`BlockCommand<R: BlockRepository>` (commands.rs) is a CQRS write-side command handler. It is **not** a use-case trait — it's a concrete struct. It handles:
- `create`, `update`, `delete`, `hard_delete`, `restore`, `set_marker`, `handle_move`

**No server handler currently uses `BlockCommand`.** Every handler constructs repos directly and does its own orchestration. The MCP layer also doesn't use `BlockCommand` — it uses `BlockUseCases` traits instead.

There's also `PageCommand<R: PageRepository>` and `SettingsCommand<R: SettingsRepository>`, neither of which is used by any handler.

**Status of commands.rs**: These are the "old" command pattern. The use-case traits (`BlockUseCases`, `PageUseCases`) have superseded them with a cleaner interface. The commands have richer signatures (hard_delete, restore, handle_move) but those operations aren't yet exposed through the use-case traits. The `create` command uses `BlockContent::from_text(content)` while the use-case trait uses raw strings — a subtle but deliberate design divergence (use cases accept raw strings and handle content creation internally).

### Approaches

#### 1. Add `AppServices` to `AppState` (Recommended)
Add one field: `app_services: AppServices` to `AppState`. Build it in `main.rs` where `pool` is already available. Handlers extract `AppServices` via `Extension(app_services)` (or `FromRef`) and call the appropriate use case.

- **Pros**: Uses existing `AppServices` struct (designed exactly for this). Single field to add. Minimal wiring in main.rs. Removes `pool` from AppState — cleans up the deprecated field.
- **Cons**: Handlers get all 4 use cases even if they only need one. Granularity is coarser than individual fields.
- **Effort**: Medium (wire once in main.rs, update ~8 handler files)

#### 2. Add individual use-case fields to `AppState`
Replace `pool` with individual `Arc<dyn BlockUseCases>`, `Arc<dyn PageUseCases>`, etc. Each handler extracts only what it needs.

- **Pros**: Matches the "Extension Registry" philosophy in state.rs comments. Fine-grained dependency injection. Clearer what each handler depends on. FromRef per trait object.
- **Cons**: More fields in AppState (4-6 new fields). More FromRef impls needed. Requires constructing all use cases in main.rs (similar wiring to Option 1).
- **Effort**: Medium-High (more wiring and FromRef impls, but same handler changes)

#### 3. Hybrid: `AppServices` in `AppState` + keep `ref_service` separate
Add `AppServices` to AppState but keep domain-specific services (`RefService`, `EventBus`, `NavigationChannel`) as separate fields. Use `FromRef` for both.

- **Pros**: Uses AppServices for CRUD use cases. Keeps cross-cutting services separate. Minimal structural change. `RefService` stays as `Arc<RwLock<>>` which is how both handlers and WebSocket need it.
- **Cons**: Two extraction points instead of one.
- **Effort**: Medium (same wiring, same handler changes — just a different field name)

### Recommendation

**Approach 1: Add `AppServices` to `AppState`.**

This is the cleanest path because:
1. `AppServices` already exists and is designed as the composition root
2. The MCP layer pattern (build repos once, wrap in use cases, inject) proves this works
3. We can remove the deprecated `pool` field from AppState (zero runtime panics — all handlers switch to use cases)
4. `ref_service`, `navigation_channel`, and `event_bus` stay as separate extensions — they're not use-case concerns
5. The templates handler already follows this pattern (just needs the instantiation moved to startup)

**The `BlockCommand` / `PageCommand` structs** should NOT be used — they're superseded by the use-case traits. Keep them for backward compat until all handlers migrate, then consider removing.

### Migration Plan (high-level)

**Step 1: Wire AppServices in main.rs**
```rust
let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
let tag_repo = Arc::new(SqliteTagRepository::new(pool.clone()));

let app_services = AppServices::new(
    Arc::new(BlockUseCasesImpl::new(block_repo.clone(), page_repo.clone())),
    Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone())),
    Arc::new(SearchUseCasesImpl::new()
        .with_search_service(Arc::new(SearchService::new(Arc::new(pool.clone()))))
        .with_block_repo(block_repo.clone())),
    Arc::new(ResourceUseCasesImpl::new(block_repo, page_repo, tag_repo)),
);

let state = AppState::new_with_services(pool, search_index, ref_service, app_services);
```

**Step 2: Add `FromRef<AppState> for AppServices`** in state.rs

**Step 3: Migrate handlers one by one**
- `blocks.rs` → `BlockUseCases` (create, delete, link, backlinks, list-by-author)
- `pages.rs` → `PageUseCases` + `BlockUseCases` (CRUD, journal, get_blocks)
- `search.rs` → `SearchUseCases`
- `graph.rs` → `BlockUseCases` + `PageUseCases` (subgraph traversal)
- `navigate.rs` → `PageUseCases` (page lookup)
- `references.rs` → `BlockUseCases` + `RefService` (stays separate)
- `query.rs` → `SearchUseCases`
- `migration.rs` → `BlockUseCases` + `PageUseCases`
- `properties.rs` → `BlockUseCases` (property CRUD is block updates)

**Step 4: Cache template use cases** — move `build_use_cases()` from per-request to startup

**Step 5: Remove deprecated `pool` from AppState** after all handlers migrated

### Risks

- **Missing operations in use-case traits**: Some handlers do things not covered by current use-case traits (e.g., `handle_move` in commands.rs, `count_by_page`, `query_dsl` raw SQL). The `graph.rs` BFS traversal constructs repos directly — it needs both `BlockRepository` and `RefService`. Solution: add needed methods to use-case traits, or keep `ref_service` as a separate extension for cross-cutting operations.
- **`search_blocks` in `blocks.rs`** uses `SearchService` directly (not `SearchUseCases`). The `SearchUseCases` trait already wraps `SearchService` — should be a straightforward migration.
- **`references.rs`** uses `SqliteRefRepository` directly, which requires `pool`. This handler should use `RefService` (already in AppState as `Arc<RwLock<RefService>>`).
- **No `SettingsUseCases` trait** — settings handler uses `SqliteSettingsRepository` directly. Either add a `SettingsUseCases` trait or wire via `SettingsCommand` (less ideal).
- **DTO mapping stays in handlers** — the `BlockDto`, `PageDto`, etc. are presentation-layer concerns and should remain in handlers. Use cases return domain objects (Block, Page) — handlers convert to DTOs. This is correct Clean Architecture.

### Ready for Proposal

**Yes.** The path is clear. The MCP layer proves the pattern works. The `AppServices` struct is ready. The deprecated `pool` field in AppState was put there explicitly waiting for this migration. The templates handler already uses `TemplateUseCases` — we just need to extend that pattern to all other handlers.

One tactical question for the proposal: should `graph.rs`'s `bfs_subgraph` function move into a use case, or stay in the handler with `BlockRepository` + `RefService` injected separately? The function is pure (no HTTP concerns) and would benefit from being in the application layer for unit testing, but it uses `RefService` which is a service, not a repository. This is worth a brief discussion in the proposal phase.
