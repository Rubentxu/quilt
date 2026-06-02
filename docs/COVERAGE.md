# Code Coverage Report

> **Last updated**: 2026-06-02
>
> Generated with [`cargo-llvm-cov` 0.8.7](https://github.com/taiki-e/cargo-llvm-cov) on stable Rust.
> Refresh by running `just coverage` (or `cargo llvm-cov --workspace --summary-only --ignore-run-fail`).

## Summary

| Metric   | Coverage |
|----------|----------|
| Regions  | 86.35% (4 062 / 29 750 missed) |
| Lines    | 84.61% (2 873 / 18 665 missed) |
| Functions| 74.95% (692 / 2 762 missed) |
| Branches | not reported (no branch data) |

Per-crate line coverage (averaged across files inside the crate):

| Crate                | Lines covered | Total | %    |
|----------------------|---------------|-------|------|
| `quilt-bin`          | 0 / 23        | 23    | 0.0% |
| `quilt-platform`     | 93 / 257      | 257   | 36.2% |
| `quilt-server`       | 970 / 1 452   | 1 452 | 66.8% |
| `quilt-mcp`          | 413 / 604     | 604   | 68.4% |
| `quilt-test-helpers` | 334 / 435     | 435   | 76.8% |
| `quilt-domain`       | 1 584 / 2 045 | 2 045 | 77.5% |
| `quilt-application`  | 438 / 549     | 549   | 80.0% |
| `quilt-query`        | 1 007 / 1 228 | 1 228 | 82.1% |
| `quilt-infrastructure` | 1 277 / 1 456 | 1 456 | 87.8% |
| `quilt-analysis`     | 2 379 / 2 704 | 2 704 | 88.0% |
| `quilt-core`         | 6 125 / 6 712 | 6 712 | 91.3% |
| `quilt-search`       | 1 167 / 1 200 | 1 200 | 97.3% |

The full per-file table is captured in [`coverage/summary.txt`](../coverage/summary.txt)
and the HTML drill-down lives at [`coverage/html/index.html`](../coverage/html/index.html).

> **Note on `--ignore-run-fail`**: the workspace currently ships four
> pre-existing broken property-based integration tests that prevent `cargo test`
> from completing cleanly:
>
> - `crates/quilt-domain/tests/order_proptest.rs` — references
>   `quilt_domain::services::order_utils` which exists in `src/` but is not
>   declared in `services/mod.rs`.
> - `crates/quilt-domain/tests/journal_day_proptest.rs` — assumes
>   `JournalDay` implements `PartialOrd` (it does not).
> - `crates/quilt-search/tests/sanitize_proptest.rs` — references a removed
>   symbol.
> - `crates/quilt-core/tests/parser_proptest.rs` — runtime panics on several
>   cases.
>
> `--ignore-run-fail` is used to keep `cargo llvm-cov` from aborting the whole
> report when these targets fail. Coverage of the rest of the workspace is
> unaffected. Fixing these tests is tracked separately.

## Low-coverage areas

### Files at 0% line coverage

These files have no exercised paths in the current test suite. Most are wiring,
boilerplate, or trait-only code; some are genuinely untested.

| File                                                  | Lines | Likely untested code |
|-------------------------------------------------------|-------|----------------------|
| `quilt-application/src/bootstrap.rs`                  | 13    | App composition root — wired in production, not invoked from tests |
| `quilt-bin/src/main.rs`                               | 23    | CLI entrypoint — only exercised through the server |
| `quilt-core/src/query/ast.rs`                         | 11    | Re-export / type-only module |
| `quilt-core/src/types.rs`                             | 6     | Type-only module |
| `quilt-domain/src/entities/journal.rs`                | 34    | Domain entity (no direct test; date logic lives in `value_objects/journal_day.rs`) |
| `quilt-domain/src/repositories/*.rs` (4 files)        | 17    | Trait definitions only — implementations are tested in `quilt-infrastructure` |
| `quilt-mcp/src/serialization.rs`                      | 17    | Manual JSON helpers — most paths covered indirectly through `handlers/*` |
| `quilt-server/src/handlers/frontend.rs`               | 106   | Static-file serving — only smoke-tested by booting the binary |
| `quilt-server/src/handlers/metrics.rs`                | 36    | Prometheus exporter — needs the `/metrics` route exercised |
| `quilt-server/src/handlers/websocket.rs`              | 100   | WS broadcast path — needs an integration test that connects a client |
| `quilt-server/src/main.rs`                            | 76    | Server entrypoint (separate from `quilt-bin`) |

### Files with partial coverage (next 10 lowest)

These represent the highest-leverage gaps. Many are pure logic and should be
easy to push above 80%.

| Lines% | File                                                          | Likely untested code |
|-------:|---------------------------------------------------------------|----------------------|
| 10.71  | `quilt-query/src/dialect.rs`                                 | SQL dialect quirks — only the generic path is exercised |
| 16.22  | `quilt-application/src/use_cases/block.rs`                   | Block use-case orchestration paths |
| 17.24  | `quilt-server/src/handlers/navigate.rs`                      | WebSocket navigate broadcast — error and broadcast branches |
| 21.88  | `quilt-domain/src/errors/domain_error.rs`                    | `From` conversions for less-common error types |
| 29.17  | `quilt-application/src/use_cases/resource.rs`                | Resource use-case error paths |
| 35.00  | `quilt-domain/src/value_objects/priority.rs`                  | Lower-priority tiers (`[#C]`, `[#D]`) and conversions |
| 36.19  | `quilt-platform/src/cli.rs`                                  | CLI parse branches — only happy paths covered |
| 45.53  | `quilt-domain/src/services/outliner_service.rs`              | Tree rebalance, indent conversion edge cases |
| 48.62  | `quilt-domain/src/value_objects/property_value.rs`           | Less-common property types (DateTime, Asset, Number conversions) |
| 53.57  | `quilt-mcp/src/handlers/resource.rs`                         | MCP resource read/serialize branches |

## How to read

- **Lines**: % of source code lines executed at least once during the test run.
- **Regions**: contiguous groups of executable code — a more granular view that
  catches partial-line gaps.
- **Functions**: % of functions called at least once. A low function-coverage %
  with high line coverage % usually means a few large helper functions that
  are never called.
- **Branches**: `if` / `match` arm coverage. Not currently reported by
  `cargo-llvm-cov` for this build; consider enabling with `--branch` if a
  branch gap analysis is needed.

## Goals

| Layer                      | Target | Current | Status |
|----------------------------|--------|---------|--------|
| Domain (`quilt-domain`)    | 90%+   | 77.5%   | Below target — see partial-coverage table above |
| Application (`quilt-application`) | 80%+ | 80.0% | At target |
| Infrastructure (`quilt-infrastructure`) | 70%+ | 87.8% | Above target |
| Server (`quilt-server`)    | 80%+   | 66.8%   | Below target — mostly wiring + WS/metrics handlers |
| Core/WASM (`quilt-core`)   | 75%+   | 91.2%   | Above target |
| Analysis (`quilt-analysis`)| 70%+   | 88.0%   | Above target |
| Search (`quilt-search`)    | 70%+   | 97.3%   | Above target |

Coverage is a useful signal, but it is not the only one. 100% line coverage
does not guarantee correctness — it only proves every line was executed once.
Use coverage to spot *untested* code, then judge whether that code deserves
tests based on its complexity and likelihood of regression.

## How to generate

```bash
# One-off
cargo install cargo-llvm-cov            # install the tool
rustup component add llvm-tools         # add the llvm-tools component
just coverage                           # run the full report

# CI
just coverage-ci                        # LCOV + summary only (faster in pipelines)
```

Artifacts produced (all gitignored except `docs/COVERAGE.md`):

- `coverage/summary.txt` — region/function/line table
- `coverage.txt` — per-file text report
- `coverage/html/index.html` — browsable HTML report
- `coverage/lcov.info` — LCOV for Codecov / Coveralls / SonarQube
