# Proposal: journal-editing-and-config

## Intent

Journal pages are read-only — JournalView lacks the editing infrastructure (SelectionState, DragState, PageOutliner) that PageView provides. Block content edits are lost on navigation because `on_save` is local-only. BlockDto is defined independently in 3 crates (quilt-ui, quilt-server, quilt-http), violating DDD single-source-of-truth. The `journal_format` setting exists but is never read. This change unifies journal editing via PageView, consolidates BlockDto, fixes persistence, and wires the date format setting end-to-end.

## Scope

### In Scope
- Route `/journal/:date` through PageView (with date-derived page name)
- Canonical BlockDto in `quilt-application`; retire duplicates in quilt-ui, quilt-server, quilt-http
- Block `on_save` → `bridge::update_block` (server persistence on blur)
- `JournalDay::to_formatted(&str)` using chrono strftime
- Settings REST API (GET/PUT `/api/v1/settings`) + `SqliteSettingsRepository` impl

### Out of Scope
- BlockDto shared crate (stays in quilt-application)
- CRDT sync / conflict resolution
- Multiple date formatter UI selector (text input only)
- Undo/redo for journal edits

## Capabilities

### New Capabilities
- `journal-date-format`: user-configurable journal date format for display titles (strftime)
- `block-autosave`: block content persisted to server on blur/navigation
- `settings-api`: REST endpoints to read/write UserSettings

### Modified Capabilities
- `journal-page`: uses PageView editing infrastructure instead of read-only JournalView
- `block-editor`: on_save callback now calls bridge::update_block

## Approach

### Journal Unification
Route `/journal/:date` resolves to PageView with `page_name = JournalDay::to_formatted(date, journal_format)`. JournalView reduced to date-navigation wrapper. Route params stay ISO for URL stability.

### BlockDto DDD Refactor
Canonical `BlockDto` in `quilt-application/src/commands.rs` (or new `dtos.rs`). quilt-server and quilt-http handlers convert `Block` ↔ application `BlockDto`. quilt-ui bridge converts via application `BlockDto`. Three duplicate definitions retired.

### on_save Server Persistence
Block component's `on_save` calls `bridge::update_block(id, content)` on blur. `bridge::update_block` already exists (PUT `/api/v1/blocks/:id`). No new endpoint needed.

### journal_format End-to-End
`JournalDay::to_formatted(format: &str)` uses `NaiveDate::format()`. Backend reads UserSettings for display title. Frontend settings signal provides format to components.

### Settings REST API
- `GET /api/v1/settings` → `UserSettings` JSON
- `PUT /api/v1/settings` → update `UserSettings`
- `SqliteSettingsRepository` impl in `quilt-infrastructure/src/database/sqlite/repositories.rs`

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/quilt-ui/src/pages/journal.rs` | Modified | Remove JournalView, add date→page_name resolver |
| `crates/quilt-ui/src/pages/page.rs` | Modified | Accept journal-aware routing |
| `crates/quilt-ui/src/components/block.rs` | Modified | Wire on_save → bridge::update_block |
| `crates/quilt-ui/src/bridge.rs` | Modified | Retire local BlockDto, import from application |
| `crates/quilt-ui/src/app.rs` | Modified | Route `/journal/:date` through PageView |
| `crates/quilt-application/src/commands.rs` | Modified | Canonical BlockDto definition |
| `crates/quilt-server/src/handlers/blocks.rs` | Modified | Convert server Block ↔ application BlockDto |
| `crates/quilt-http/src/handlers/blocks.rs` | Modified | Convert HTTP Block ↔ application BlockDto |
| `crates/quilt-server/src/handlers/mod.rs` | Modified | Add settings handler registration |
| `crates/quilt-domain/src/value_objects/journal_day.rs` | Modified | Add `to_formatted(&str)` method |
| `crates/quilt-infrastructure/src/database/sqlite/repositories.rs` | Modified | Add SqliteSettingsRepository impl |
| `crates/quilt-ui/src/wasm/bindings.rs` | Modified | Retire local BlockDto if applicable |

## Entropy Budget (Protocol B)

**Method**: Heuristic

| Metric | Estimate | Threshold | Status |
|--------|----------|-----------|--------|
| H(Δ_existing) | ~3.2 bits (12 files) | < 1.0 | ❌ OCP violation (expected for unification refactor) |
| H(Δ_new) | ~2.0 bits (settings API + to_formatted) | > 0 | ✅ |
| New connascence pairs | 2 (application BlockDto → adapters) | < 3 | ✅ |
| OCP compliant? | No (unification requires existing changes) | — | Accepted |

**Breaking Change Indicators**: BlockDto consolidation touches 3 crates — all consumers must be migrated atomically.

**Verdict**: yellow — OCP violation is inherent to the DDD consolidation refactor. Net entropy improvement: DQS 0.53 → ~0.70+ (eliminates 3-way BlockDto duplication at I=5.6 bits).

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| BlockDto field mismatch across 3 crates | Medium | Audit all 3 definitions before migration |
| Settings table migration needed | Low | Auto-create on first read if absent |
| JournalView removal breaks g-j/g-p keybindings | Low | Test keybindings after route change |

## Rollback Plan

```bash
git revert HEAD -- "crates/quilt-ui/**" "crates/quilt-application/**" "crates/quilt-server/**" "crates/quilt-http/**" "crates/quilt-domain/**" "crates/quilt-infrastructure/**"
```

No destructive schema changes. Settings table can be dropped if needed.

## Dependencies

- `SqliteSettingsRepository` must exist before settings REST API
- `JournalDay::to_formatted` must exist before journal format wiring

## Success Criteria

- [ ] Journal page is editable (blocks can be typed, saved, created)
- [ ] Editing a block persists to server (verified via page reload)
- [ ] Date format setting changes journal title display
- [ ] BlockDto is defined once in quilt-application
- [ ] `cargo test` passes; `cargo clippy` clean
