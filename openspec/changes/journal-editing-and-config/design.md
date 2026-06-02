# Design: Journal Editing and Configuration

## Technical Approach

Unify journal pages into PageView to inherit editing infrastructure (SelectionState, DragState, PageOutliner). Wire Block `on_save` → `bridge::update_block` (already calls PATCH /api/v1/blocks/{id}). Consolidate 4 BlockDto definitions into one canonical DTO in `quilt-application/src/use_cases/dtos.rs`. Create `SqliteSettingsRepository` impl and REST GET/PUT `/api/v1/settings` for `journal_format` end-to-end. Add `JournalDay::to_formatted(&str)` for display—separate from Display (ISO) for DB consistency.

## Architecture Decisions

### Decision: BlockDto canonical location

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `quilt-application/src/use_cases/dtos.rs` (new) | New file in application layer; DTOs as use case outputs | **Chosen** |
| `quilt-domain` | Domain shouldn't know about HTTP serialization; violates DDD | Rejected |
| Keep duplicated (current state) | 4 divergent definitions; quilt-ui lacks `page_name`, quilt-server has `level: i32` vs `u8` | Rejected |

**Rationale**: Application layer owns DTOs as adapter-agnostic contracts. Domain stays pure. All 4 adapters (quilt-server, quilt-http, quilt-ui bridge, quilt-ui wasm/bindings) convert via canonical `BlockDto`.

### Decision: Journal route unification

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Route `/journal/:date` → PageView (date → page_name) | PageView already has 4 contexts; journal page = regular page with date name | **Chosen** |
| Keep JournalView + add contexts | Code duplication grows; two views to maintain forever | Rejected |

**Rationale**: Follows Logseq pattern—journal is a page with date as name. PageView provides SelectionState, DragState, PageOutliner, and zoom. JournalView currently has none—blocks are read-only.

### Decision: JournalDay formatting

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `JournalDay::to_formatted(&str)` — separate method | `Display` stays ISO for DB lookups/route params; `to_formatted` for UI only | **Chosen** |
| Modify Display impl | Breaks all ISO consumers (DB, routes, PageView page_name resolution) | Rejected |

### Decision: Settings persistence

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `SqliteSettingsRepository` in quilt-infrastructure using `config` table | `config` table already exists (key/value/updated_at); trait `SettingsRepository` already defined; `SettingsCommand` already exists | **Chosen** |
| JSON file | Adds filesystem I/O complexity; breaks existing SQLite-only stack | Rejected |
| In-memory only | Settings lost on restart | Rejected |

**Note**: `SqliteSettingsRepository` is imported in 3 crates (quilt-http, quilt-platform, quilt-bin) but has no struct definition—this change creates the impl.

## Data Flow

```
[Journal route /journal/2026-05-27]
    → PageView::page_name = "2026-05-27"
    → bridge::get_page_blocks("2026-05-27")  [GET /api/v1/pages/{name}/blocks]
    → BlockDto[] in camelCase
    → Block components render with edit, select, drag contexts

[Block on_save flow]
    Block (on_save callback, block.rs:118)
        → bridge::update_block(id, content)  [PATCH /api/v1/blocks/{id}]
        → BlockUseCases::update_content(Uuid, &str)  [application]
        → SQLite UPDATE blocks SET content = ? WHERE id = ?
        → BlockDto returned

[Settings flow]
    GET /api/v1/settings
        → SqliteSettingsRepository::get_user_settings()
        → SELECT value FROM config WHERE key = 'user_settings'
        → UserSettings JSON (includes journal_format)

    PUT /api/v1/settings { journal_format: "%B %d, %Y" }
        → UserSettings::validate()
        → SqliteSettingsRepository::update_user_settings()
        → INSERT OR REPLACE config(key, value, updated_at)
        → JournalDay::to_formatted("%B %d, %Y") → "May 27, 2026"
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/quilt-application/src/use_cases/dtos.rs` | Create | Canonical BlockDto (id, page_id, parent_id, content, order, level: u8, marker, priority, collapsed, properties, refs, created_at, updated_at) |
| `crates/quilt-domain/src/value_objects/journal_day.rs` | Modify | Add `to_formatted(format: &str) -> String` using NaiveDate::format() |
| `crates/quilt-infrastructure/src/database/sqlite/repositories.rs` | Modify | Add `SqliteSettingsRepository` struct + impl `SettingsRepository` trait |
| `crates/quilt-server/src/state.rs` | Modify | Add `settings_repo: Arc<SqliteSettingsRepository>` to AppState |
| `crates/quilt-server/src/handlers/settings.rs` | Create | GET/PUT settings handlers using AppState |
| `crates/quilt-server/src/handlers/mod.rs` | Modify | Add `pub mod settings` |
| `crates/quilt-server/src/routes.rs` | Modify | Add GET/PUT `/api/v1/settings` routes |
| `crates/quilt-server/src/handlers/blocks.rs` | Modify | Remove local BlockDto, import from `quilt_application::use_cases::dtos` |
| `crates/quilt-http/src/handlers/blocks.rs` | Modify | Remove local BlockDto, import canonical |
| `crates/quilt-application/src/commands.rs` | Modify | Re-export BlockDto from dtos module |
| `crates/quilt-ui/src/bridge.rs` | Modify | Remove local BlockDto, import from application; add `get_settings`/`update_settings` functions |
| `crates/quilt-ui/src/wasm/bindings.rs` | Modify | Remove local BlockDto, import canonical |
| `crates/quilt-ui/src/pages/journal.rs` | Modify | Remove JournalView, add date → page_name resolver helper |
| `crates/quilt-ui/src/pages/page.rs` | Modify | Add journal date-navigation header (← Today →) |
| `crates/quilt-ui/src/app.rs` | Modify | Route `/journal/:date` → PageView; add settings signal provider |
| `crates/quilt-ui/src/components/block.rs` | Modify | Wire `on_save` → `bridge::update_block` via `spawn_local` |
| `crates/quilt-ui/src/state.rs` | Create | Settings signal (RwSignal<UserSettings>) + fetch on app mount |

## Entropy Analysis (Protocol C)

**Method**: Heuristic (code reading)

| Interface | I(X;T) Leakage | I(T;Y) Coverage | Quality |
|-----------|---------------|-----------------|---------|
| BlockDto (canonical) | LOW (~1.5 bits) | HIGH (~2.8 bits) | ✅ Optimal |
| SqliteSettingsRepository | LOW (~0.5 bits) | HIGH (~1.0 bits) | ✅ Optimal |
| Settings REST API | LOW (~1.0 bits) | HIGH (~2.0 bits) | ✅ Optimal |
| JournalDay::to_formatted | LOW (~0.2 bits) | HIGH (~0.5 bits) | ✅ Optimal |

**SOLID-Entropy**:
| Principle | Value | Threshold | Status |
|-----------|-------|-----------|--------|
| OCP | H(Δ_existing) ≈ 3.46 bits (11 files) | < 1.0 | ❌ Accepted (unification refactor) |
| SRP | `to_formatted` single-reason | — | ✅ |
| ISP | SettingsRepository: 3 methods, all used | — | ✅ |
| DIP | SettingsRepository trait (abstraction) over SQLite | — | ✅ |

**DQS**: 0.53 → ~0.70+ (eliminates 3-way BlockDto duplication at I ≈ 5.6 bits).

## Testing Strategy

| Layer | What | How |
|-------|------|------|
| Unit | JournalDay::to_formatted | Test with 6 common_format strings |
| Unit | UserSettings::validate | Existing tests pass |
| Unit | BlockDto serialize/deserialize | All 4 adapters produce/pass same JSON |
| Integration | SqliteSettingsRepository | get→update→get cycle test |
| Integration | Settings REST API | reqwest: GET returns defaults, PUT updates |
| E2E | Journal page editing | Playwright: type in block, reload, verify content persisted |
| E2E | Date format change | Playwright: PUT settings with format, navigate journal, verify title |

## Open Questions

- [ ] Should `journal_format` apply to JournalView header OR block-level date strings? (Design assumes header only)
- [ ] Do we remove `wasm/bindings.rs` BlockDto entirely or leave for WASM-specific fields? (Design: retire)
