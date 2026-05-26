# Proposal: DSL `analyze` Operator (Change B2)

## Executive Summary
This change adds a new `analyze` operator to the Quilt query DSL that runs cognitive analysis (CognitiveMirror) or serendipity discovery (SerendipityEngine) on blocks matched by a filter expression. The operator takes a separate async execution path since analysis requires async Rust code rather than SQL generation. Results are returned as full JSON (CognitiveMap or Vec<SerendipityConnection>).

## Intent

We are building a bridge between the declarative DSL filter language and the Rust-based analysis engines in `quilt-analysis`. Currently the DSL generates SQL for synchronous database queries; `analyze` introduces an async execution path that calls into CognitiveMirror and SerendipityEngine directly, returning complex typed results that cannot be expressed as SQL rows.

## User Decisions (Confirmed)

1. **Execution model**: Option A — separate `execute_analyze()` async method (MVP approach)
2. **Return type**: Option A — full JSON (CognitiveMap / Vec<SerendipityConnection>)
3. **Nesting**: Option A — prohibited (no nesting, top-level only)
4. **Filter interpretation**: Option A — analyze blocks (inner filter returns blocks, analyze runs on those blocks)

## Scope

### In scope
- **Parser** (`quilt-query/src/parser.rs`): Add `AnalyzeKind` enum, `QueryExpr::Analyze` variant, `parse_analyze()` method
- **Executor** (`quilt-query/src/executor.rs`): Add `AnalyzeResult` enum, async `execute_analyze()` method
- **MCP**: New DSL tool handler or extend existing `quilt_query` to handle analyze expressions with async path
- **Tests**: Parser unit tests, executor tests with mock AnalysisEngine, integration tests

### Out of scope
- Full DSL refactor to unified async executor (Option B from exploration)
- Nesting of `analyze` inside other operators
- Analysis over pages directly (only via filter-to-blocks-to-page path)

## Approach

### Step 1: Parser changes
1. Add `AnalyzeKind` enum with variants: `CognitiveMirror`, `Serendipity { limit, min_confidence, temporal_window_days }`
2. Add `QueryExpr::Analyze { inner: Box<QueryExpr>, kind: AnalyzeKind }` variant
3. Add `parse_analyze()` method to parse `(analyze ...)` s-expressions
4. Parse keyword args: `:limit N`, `:min-confidence F`, `:temporal-window-days N`

### Step 2: Executor changes
1. Add `AnalyzeResult` enum with variants: `CognitiveMap(CognitiveMap)`, `SerendipityConnections(Vec<SerendipityConnection>)`
2. Add `pub async fn execute_analyze(&self, expr: &QueryExpr, analysis: &AnalysisEngine) -> Result<AnalyzeResult, AnalyzeError>`
3. First evaluate inner filter to get blocks via existing sync SQL path
4. Resolve blocks to containing pages
5. Call appropriate analysis engine (CognitiveMirror or SerendipityEngine)
6. Return `AnalyzeResult` with serialized JSON

### Step 3: MCP integration
1. Extend `quilt_query` tool handler to detect `QueryExpr::Analyze` variants
2. Route to async `execute_analyze()` path with injected `AnalysisEngine`
3. Serialize `AnalyzeResult` to JSON string for tool response

### Step 4: Tests
1. Parser round-trip tests: parse `(analyze (task TODO) cognitive_mirror)` and verify reconstructed string
2. Executor tests: mock `AnalysisEngine`, verify correct engine method called with correct args
3. MCP integration test: invoke `quilt_query` with analyze expressions

## Architecture

### Async Execution Path
```
DSL string
  → parse() [sync]
  → QueryExpr::Analyze AST
  → QueryService::prepare() [sync, returns SQL + params]
  → Execute SQL [sync, gets blocks]
  → QueryService::execute_analyze() [NEW async]
  → CognitiveMirror::analyze_blocks() OR SerendipityEngine::find_connections()
  → AnalyzeResult JSON
```

### Key Design Points
- **Separate from build_sql()**: `execute_analyze()` is a new async method, not integrated into the sync `build_sql()` path
- **Filter first**: Inner expression evaluated as a standard filter to get blocks
- **Page resolution**: Blocks are resolved to their containing page(s) for analysis
- **Hybrid return**: Results serialized as JSON strings (matching MCP protocol)

### Module Structure
```
quilt-query/src/
  parser.rs       ← AnalyzeKind, QueryExpr::Analyze, parse_analyze()
  executor.rs     ← AnalyzeResult, execute_analyze() async method
  lib.rs          ← Export AnalyzeKind, AnalyzeResult

quilt-application/src/
  query_service.rs ← execute_analyze() bridge

quilt-mcp/src/
  server.rs       ← Route analyze to async path in quilt_query tool
```

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-------------|--------|------------|
| Circular dependency: quilt-query gaining async deps | High | High | Keep quilt-query sync-only; async logic lives in quilt-application or quilt-mcp |
| Page vs blocks ambiguity | Medium | Medium | Document that analyze first filters blocks, then runs on containing page |
| Analysis engine availability | Low | Medium | AnalysisEngine injected at runtime; return clear error if not configured |
| SerendipityQuery construction from DSL args | Low | Low | Map DSL args directly to SerendipityQuery fields |

## Open Questions

None — all resolved by user decisions.

## Dependencies

- Rust 2024 edition with async support
- `quilt-analysis` crate (CognitiveMirror, SerendipityEngine, CognitiveMap, SerendipityConnection types)
- `quilt-query` crate (parser + executor)
- `quilt-application` crate (QueryService bridge)
- `quilt-mcp` crate (tool integration)
- Async execution context (Tokio)

## Success Criteria

1. **Parser**: `(analyze (task TODO) cognitive_mirror)` and `(analyze (page "X") serendipity :limit 20)` parse without error
2. **Executor**: `execute_analyze()` calls correct engine method and returns appropriate result type
3. **MCP**: `quilt_query` tool accepts analyze expressions and returns JSON serialized results
4. **Tests pass**: `cargo test -p quilt-query` runs green
5. **No clippy warnings**: `cargo clippy -p quilt-query` reports zero warnings
6. **Lints pass**: `cargo fmt --check` passes
