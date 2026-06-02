# Test Strategy — Quilt

**Last updated**: 2026-06-02
**Owner**: Quilt Team

## Stack

Rust workspace (14 crates, Clean/Hexagonal architecture) with `axum` + `sqlx` (SQLite) + `tokio`. Frontend React 19 + TypeScript + Vite + Tailwind CSS 4. E2E with Playwright + axe-core for a11y. Test runners: `cargo test`, `vitest`, `playwright test`. Fixtures: `quilt-test-helpers` (in-memory repos). Property testing: `proptest`.

## Current state (2026-06-02)

| Layer | Count | Status |
|---|---|---|
| Unit (Rust) | ~1,000 tests | ✅ Solid in domain/core/analysis. Gaps in application/platform. |
| Component (React) | 90 tests (10 files) | 🟡 15% of 46 source modules covered. |
| Integration (Rust) | ~200 tests | 🟡 Server API good. MCP handlers weak. DB repos decent. |
| E2E (Playwright) | 12 specs | ✅ Covers smoke, outliner, navigation, search, theme, journal, a11y, visual. |

Coverage: 86.35% region, 84.61% lines (Rust).

## Target shape

```
        E2E     ████  (~10 specs, one per critical user journey)
   API/Int     ██████  (all public handlers, MCP tools, use cases)
  Component    ██████  (all hooks, key components, utilities)
    Unit       ██████████  (every value object, entity, error variant, parser)
```

## Roadmap

### 🔴 Phase 4 — Server & MCP handlers (priority: CRITICAL)

| # | Task | Files | Effort | Value |
|---|---|---|---|---|
| 4.1 | Fix `create_task` bug (insert before update with priority) | `quilt-application/src/use_cases/block.rs` | S | Bug fix |
| 4.2 | Integration tests for MCP block handler with mock `BlockUseCases` | `crates/quilt-mcp/tests/block_handler_tests.rs` | M | Alto |
| 4.3 | Integration tests for MCP page/resource handlers | `crates/quilt-mcp/tests/page_handler_tests.rs` | M | Alto |
| 4.4 | `PageDto` and `BlockDto` conversion roundtrip tests | `crates/quilt-server/tests/dto_tests.rs` | S | Medio |
| 4.5 | `use_cases/resource.rs` — graph snapshot, list pages, tags | `crates/quilt-application/tests/resource_use_cases_tests.rs` | M | Alto |

### 🟠 Phase 5 — Frontend hooks & utilities (priority: HIGH)

| # | Task | Files | Effort | Value |
|---|---|---|---|---|
| 5.1 | `useSSE` — EventSource connection, reconnect, event parsing | `quilt-ui/.../useSSE.test.ts` | M | Alto |
| 5.2 | `api-client` — request building, error handling, auth header | `quilt-ui/.../api-client.test.ts` | M | Alto |
| 5.3 | `useResponsive` — breakpoint detection via useMediaQuery | `quilt-ui/.../useResponsive.test.ts` | S | Bajo |
| 5.4 | `flattenTree` — edge cases with deep nesting, empty sets | (existing file, add cases) | S | Medio |
| 5.5 | `WasmProvider` — context value, loading state | `quilt-ui/.../WasmProvider.test.tsx` | M | Medio |

### 🟡 Phase 6 — CLI & Server gaps (priority: MEDIUM)

| # | Task | Files | Effort | Value |
|---|---|---|---|---|
| 6.1 | CLI argument parsing tests (clap) | `crates/quilt-platform/tests/cli_tests.rs` | M | Medio |
| 6.2 | `navigate_to_page` / `navigate_to_block` integration tests | `crates/quilt-server/tests/navigate_integration_tests.rs` | L | Medio |
| 6.3 | WebSocket handler integration test | `crates/quilt-server/tests/websocket_tests.rs` | L | Alto |
| 6.4 | `auto-grill-loop` on MCP handlers for edge case discovery | `docs/grill/` | L | Alto |

### 🟢 Phase 7 — Property tests & fuzzing (priority: LOW)

| # | Task | Files | Effort | Value |
|---|---|---|---|---|
| 7.1 | Property tests for `PropertyValue::from_json` / `to_json` roundtrip | `crates/quilt-domain/tests/property_value_proptest.rs` | M | Medio |
| 7.2 | Property tests for `BlockFormat` parsing | `crates/quilt-domain/tests/block_format_proptest.rs` | S | Bajo |
| 7.3 | Property tests for `parse_properties` with random JSON maps | `crates/quilt-domain/tests/parse_properties_proptest.rs` | M | Medio |

## Commands (canonical)

| Need | Command |
|---|---|
| Run all unit + integration | `cargo test` |
| Run a single test | `cargo test -p <crate> --test <name> <test_fn>` |
| Run with coverage | `scripts/coverage.sh` |
| Run frontend tests | `cd quilt-ui && npx vitest run` |
| Run E2E smoke | `npx playwright test --grep @smoke` |
| Run E2E full | `npx playwright test` |

## Per-layer rules

- **Unit**: pure functions, no IO, deterministic. `#[cfg(test)]` next to code or `tests/` for public API.
- **Component**: React Testing Library, `@testing-library/user-event`, no real network.
- **Integration**: in-memory repos (`quilt-test-helpers`) for use cases; axum `TestClient` for HTTP handlers.
- **E2E**: Playwright with `getByRole`/`getByLabelText`/`getByText`. No `waitForTimeout`. Real stack, mocked third parties.

## Coverage gates

- Unit: ≥ 80% lines per crate.
- Component: ≥ 80% lines per utility/hook file.
- Integration: every public handler has at least one happy-path + one error test.
- E2E: one spec per critical user journey (outliner, journal, search, navigation, settings, graph).

## Forbidden patterns

- `waitForTimeout` / `time.sleep` / `setTimeout` in tests.
- Real network, real DB, real third-party APIs in unit / component.
- Snapshot tests for entire pages (component-level only).
- Test that depends on test execution order.
- `mock_called()` assertions that prove nothing about the user.
- CSS selectors in Playwright — use `getByRole` / `getByLabelText` / `getByText`.

## Edge-case discovery

- For new modules/services: run `auto-grill-loop` with topic from `docs/grill/`.
- For bug fixes: write the regression test first, then fix.
- For coverage audits: use the test-pyramid skill checklist.

## Bug registry

| ID | Description | File | Test |
|---|---|---|---|
| BUG-001 | `create_task` with `priority` skips `insert()` before `update()` — causes `BlockNotFound` | `quilt-application/src/use_cases/block.rs:209-220` | `test_create_task_with_priority_known_bug` |

## Change log

- 2026-06-02 — Initial strategy doc + Phase 1-3 implementation (+162 tests) — test-pyramid-builder
- 2026-06-02 — Bug BUG-001 discovered and documented — test-pyramid-builder
- 2026-06-02 — Roadmap phases 4-7 defined — test-pyramid-builder
