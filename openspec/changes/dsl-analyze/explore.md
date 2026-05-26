# Exploration: `analyze` DSL Operator (Change B2)

## Current State

### Analysis Engine Interfaces Discovered

#### CognitiveMirror (`quilt-analysis/src/cognitive_mirror/engine.rs`)
```rust
pub struct CognitiveMirror {
    block_repo: Arc<dyn BlockRepository>,
}

impl CognitiveMirror {
    // Takes a page_id, returns CognitiveMap
    pub async fn analyze(&self, page_id: Uuid) -> Result<CognitiveMap, CognitiveError>
    
    // Takes blocks directly, returns CognitiveMap
    pub async fn analyze_blocks(&self, blocks: &[Block]) -> CognitiveMap
}
```

Returns `CognitiveMap`:
- `clusters: Vec<KnowledgeCluster>` — detected knowledge clusters
- `density: HashMap<Uuid, f32>` — reference density per block
- `frontiers: Vec<Uuid>` — knowledge frontier blocks
- `gaps: Vec<KnowledgeGap>` — structural gaps between blocks
- `influences: Vec<InfluenceScore>` — centrality scores

#### SerendipityEngine (`quilt-analysis/src/serendipity/engine.rs`)
```rust
pub struct SerendipityEngine {
    block_repo: Arc<dyn BlockRepository>,
    cache: TimedCache,
    options: SerendipityOptions,
}

impl SerendipityEngine {
    pub async fn find_connections(
        &self,
        query: SerendipityQuery,
    ) -> Result<Vec<SerendipityConnection>, SerendipityError>
}
```

Returns `Vec<SerendipityConnection>`:
- `idea_a, idea_b: Uuid` — connected blocks
- `confidence: f32` — connection strength (0-1)
- `connection_type: ConnectionType` — Structural/Temporal/Content/Semantic
- `explanation: String`

#### Query Interface
- **CognitiveMirror**: Takes `page_id: Uuid` (single page analysis)
- **SerendipityEngine**: Takes `SerendipityQuery` with `page_id` OR `temporal_window_days`

### MCP Integration (`quilt-mcp/src/server.rs`)
```rust
// Lines 1116-1137: tool_cognitive_mirror
async fn tool_cognitive_mirror(&self, args: &serde_json::Value) -> Result<String, String> {
    let page_name = args.get("page_name").and_then(|v| v.as_str())...?;
    let pages = self.page_repo.get_all().await?;
    let page = pages.iter().find(|p| p.name == page_name)?;
    let map = mirror.analyze(page.id).await?;
    Ok(serde_json::to_string_pretty(&map)?)
}

// Lines 1139-1179: tool_serendipity
async fn tool_serendipity(&self, args: &serde_json::Value) -> Result<String, String> {
    let query = SerendipityQuery { ... };
    let connections = engine.find_connections(query).await?;
    Ok(serde_json::to_string_pretty(&connections)?)
}
```

**Key observation**: Both tools are `async` and access analysis engines via `Arc<dyn>` injection.

### DSL Structure (B1 - dsl-aggregates)
- **Parser** (`quilt-query/src/parser.rs`): `QueryExpr` enum with variants
- **Executor** (`quilt-query/src/executor.rs`): `build_sql()` returns `(String, Vec<SqlParam>)` — **sync only**
- **QueryService** (`quilt-application/src/query_service.rs`): `prepare()` → `QueryResult { sql, params, ast }`

---

## Affected Areas

| File | Why Affected |
|------|-------------|
| `crates/quilt-query/src/parser.rs` | Need new `analyze` parsing logic |
| `crates/quilt-query/src/executor.rs` | `build_sql()` is sync — cannot handle async `analyze` |
| `crates/quilt-query/src/lib.rs` | Need to export new types |
| `crates/quilt-application/src/query_service.rs` | `prepare()` returns SQL — doesn't handle analysis |
| `crates/quilt-mcp/src/server.rs` | Already has analysis engines injected; DSL query tool is separate |

---

## Critical Architectural Finding

### The Async Problem

The existing DSL executor pattern is:
```
DSL string → parse() → QueryExpr AST → build_sql() → (SQL, params)
```

`build_sql()` is **synchronous** and returns SQL strings. The DB executes SQL synchronously.

For `analyze`, we need:
```
DSL string → parse() → QueryExpr AST → EXECUTE ASYNC → (CognitiveMap | SerendipityResult)
```

**The `build_sql()` model breaks down because**:
1. `CognitiveMirror::analyze()` requires `&self` + `page_id` + async block_repo
2. `SerendipityEngine::find_connections()` requires `&self` + `SerendipityQuery` + async block_repo
3. These cannot be expressed as SQL — they are Rust async functions returning complex types

### Two Possible Execution Models

#### Option A: Separate `build_analyze()` Path (Recommended for MVP)
Keep `build_sql()` for sync operations. Add a separate async method:

```rust
impl QueryExecutor {
    pub fn build_sql(&self, expr: &QueryExpr, limit: usize) -> (String, Vec<SqlParam>)
    
    pub async fn execute_analyze(
        &self,
        expr: &QueryExpr,
        analysis: &AnalysisEngine,
    ) -> Result<AnalyzeResult, AnalyzeError>
}
```

**Pros**: Minimal changes to existing sync path
**Cons**: Hybrid sync/async API feels inconsistent

#### Option B: Unified Async Executor
Refactor `QueryExecutor` to be async-capable:

```rust
pub trait QueryExecutor: Send + Sync {
    async fn execute(&self, expr: &QueryExpr, ctx: &dyn QueryContext) -> Result<QueryResult, QueryError>;
}
```

**Pros**: Consistent async-first design
**Cons**: Large refactor, breaks existing sync callers

---

## DSL Integration Design

### Proposed Syntax

```lisp
(analyze (page "Page Name") cognitive_mirror)
(analyze (page "Page Name") serendipity)
(analyze (task todo) cognitive_mirror)
(analyze (task todo) serendipity :limit 20 :min-confidence 0.3)
```

### Proposed AST Variants

```rust
pub enum AnalyzeKind {
    CognitiveMirror,
    Serendipity { 
        limit: Option<usize>,
        min_confidence: Option<f32>,
        temporal_window_days: Option<i64>,
    },
}

pub enum QueryExpr {
    // ... existing ...
    
    Analyze {
        inner: Box<QueryExpr>,        // Filter before analysis
        kind: AnalyzeKind,
    },
}
```

### Return Types

| Operator | Return Type |
|----------|-------------|
| `cognitive_mirror` | `CognitiveMap` (JSON serialized) |
| `serendipity` | `Vec<SerendipityConnection>` (JSON serialized) |

### Proposed Result Wrapper

```rust
pub enum AnalyzeResult {
    CognitiveMap(CognitiveMap),
    SerendipityConnections(Vec<SerendipityConnection>),
}

impl std::fmt::Display for AnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyzeResult::CognitiveMap(map) => {
                serde_json::write(f, map)
            }
            AnalyzeResult::SerendipityConnections(conns) => {
                serde_json::write(f, conns)
            }
        }
    }
}
```

---

## Open Questions (Critical for Spec)

### 1. Execution Model
**Decision required**: Should `analyze` have a separate async executor path, or should the entire DSL become async?

**Recommendation**: Option A — separate `build_analyze()` method initially. If `analyze` usage grows, consider Option B in a future change.

### 2. Return Type
**Question**: What should `(analyze (task todo) cognitive_mirror)` return?
- Full `CognitiveMap` JSON? (verbose but complete)
- Summary stats only? (compact but lossy)
- Just block IDs that are frontiers/gaps? (minimal)

**Recommendation**: Full JSON for MVP. Future could add `:format summary` option.

### 3. Input — Filter vs Page ID
**Question**: Does `analyze` take:
- A filter expression like `(task todo)` (current DSL style)?
- A page reference like `(page "Name")`?
- Both?

**Recommendation**: Support both — use inner expression as filter. This aligns with how `aggregate`/`group_by` work.

### 4. CognitiveMirror vs Serendipity
**Question**: Two separate operators (`cognitive_mirror`, `serendipity`) or one `analyze` with a flag?

**Recommendation**: One `analyze` operator with `kind` enum. Syntax:
```lisp
(analyze (task todo) cognitive_mirror)
(analyze (task todo) serendipity :limit 20)
```

### 5. Nesting
**Question**: Can `analyze` be nested inside other operators?
```lisp
(aggregate (analyze (page "X") cognitive_mirror) (property author) (fn count))  -- Legal?
```

**Recommendation**: NO nesting initially. Keep `analyze` as a top-level operator only. Simpler implementation and clearer semantics.

### 6. Dependency Injection
**Question**: How does the DSL executor access `CognitiveMirror`/`SerendipityEngine`?
- They are currently injected into `McpServer`, not available to `QueryExecutor`
- `QueryExecutor` is sync and has no async context

**Recommendation**: Pass `AnalysisEngine` reference to async execute method:
```rust
pub async fn execute_analyze(
    &self,
    expr: &QueryExpr,
    analysis: &AnalysisEngine,
) -> Result<AnalyzeResult, AnalyzeError>
```

---

## Effort Estimation

### Parser Changes
| Component | Effort | Notes |
|-----------|--------|-------|
| `AnalyzeKind` enum | Low | Simple enum with variants |
| `QueryExpr::Analyze` variant | Low | Struct with inner + kind |
| `parse_analyze()` method | Medium | New operator, 50-80 lines |
| Error handling | Low | Standard ParseError variants |

**Parser subtotal: Medium (~100 lines)**

### Executor Changes
| Component | Effort | Notes |
|-----------|--------|-------|
| `AnalyzeResult` enum | Low | Simple wrapper |
| `build_analyze()` async method | Medium | New async path |
| Integration with existing `QueryService` | High | Must bridge sync prepare() and async execute_analyze() |

**Executor subtotal: High (~200-300 lines)**

### MCP Integration
| Component | Effort | Notes |
|-----------|--------|-------|
| New DSL tool handler for `analyze` | Medium | Similar to existing `tool_query` |
| Analysis engine access | Low | Already injected in `McpServer` |
| Result serialization | Low | Reuse existing JSON serialization |

**MCP subtotal: Medium (~100 lines)**

### Testing
| Component | Effort | Notes |
|-----------|--------|-------|
| Unit tests for parser | Low | Similar to existing aggregate tests |
| Unit tests for executor | Medium | Need mock AnalysisEngine |
| Integration tests | Medium | Full flow through MCP |

**Testing subtotal: Medium (~150 lines)**

---

## Recommendation: Split Further

**Change B2 should be split into B2a and B2b:**

### B2a: Parser + AST for `analyze`
- Add `AnalyzeKind` enum
- Add `QueryExpr::Analyze` variant
- Add `parse_analyze()` in parser
- Pure parsing — no execution

### B2b: Executor + MCP Integration
- Add `AnalyzeResult` type
- Add async `execute_analyze()` method
- Wire up to `McpServer`
- Full integration with tests

**Rationale**:
1. B1 (aggregate/stats/group_by) was self-contained — all sync, all in `quilt-query`
2. `analyze` crosses crate boundaries AND introduces async
3. Splitting reduces risk and allows parallel work on B2a (parsing) vs B2b (execution)
4. B2a is safe to implement first without touching async machinery

---

## Entropy Analysis (Connascence Landscape)

**Method**: Heuristic (CogniCode index built but analysis engines not in graph)

### Connascence Pairs

| Component A | Component B | Connascence Type | I(bits) | Severity |
|------------|-------------|------------------|---------|----------|
| `quilt-query` (parser) | `quilt-analysis` | **Meaning** | ~2.5 | ⚠️ Medium |
| `quilt-application` (QueryService) | `quilt-analysis` | **Meaning** | ~2.0 | ⚠️ Medium |
| `quilt-mcp` (server) | `quilt-analysis` | Name | ~1.5 | ⚠️ Low |
| `QueryExpr` AST | `build_sql()` | Position | ~3.5 | ❌ High |
| `QueryExpr::Analyze` | `execute_analyze()` | Name | ~2.0 | ⚠️ Medium |

### Critical Pairs (I > 3.0 bits)

| Pair | I(bits) | Issue |
|------|---------|-------|
| `QueryExpr` variant dispatch in executor | 3.5 | **Critical** — adding `Analyze` variant requires touching every match arm in `build_sql()` and `build_where()`. Currently 3 variants handle specially; adding 1 more increases complexity. |

### Hidden Connascence (Meaning/Timing)

1. **⚠️ MEANING**: `CognitiveMirror::analyze(page_id)` takes a single page ID — but DSL filter expressions operate on block sets. If user writes `(analyze (task todo))`, does this mean "analyze pages containing todo tasks" or "analyze blocks matching todo"? **No documented assumption exists.**

2. **⚠️ TIMING**: The async nature of analysis engines means results are non-deterministic (cached but TTL-dependent). SQL queries are always consistent snapshots. **This timing difference is not documented.**

### Coupling Score

- **External coupling**: H(`quilt-query` → `quilt-domain`) = Low (only uses domain types)
- **Internal coupling**: H(`executor.rs` dispatch) = High (match on QueryExpr is O(n))
- **Analysis coupling**: H(`McpServer` → `AnalysisEngine`) = Medium (optional injection)

**Estimation Method**: Heuristic
**Confidence**: estimated

---

## Risks

1. **Circular dependency risk**: `quilt-query` currently has NO async deps. Adding `quilt-analysis` would create a dep on async crate. **Mitigation**: Keep `quilt-query` sync-only; put async logic in `quilt-application` or `quilt-mcp`.

2. **Executor dispatch complexity**: Every new `QueryExpr` variant requires updating `build_sql()` match arms. This is **High connascence**. **Mitigation**: Consider exhaustive matching with `#[derive(Enumerate)]` or separate handler traits.

3. **Semantic ambiguity**: "analyze what?" — the interpretation of filter expressions for analysis is underspecified. **Mitigation**: Define clearly in spec that `analyze` first evaluates inner filter to get blocks, then runs analysis on the containing pages.

4. **Testing complexity**: Mocking async analysis engines requires `Arc<dyn BlockRepository>` + `Arc<dyn AnalysisEngine>`. **Mitigation**: Create a `MockAnalysisEngine` test helper.

---

## Ready for Proposal

**No — requires user decisions on:**
1. Execution model (Option A vs B)
2. Return type (full JSON vs summary)
3. Nesting policy (allowed vs prohibited)
4. Filter interpretation (blocks vs pages)

**Orchestrator should ask user to resolve these 4 open questions before proceeding to spec.**
