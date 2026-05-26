# Spec: DSL `analyze` Operator (Change B2)

## Overview

The `analyze` operator runs cognitive analysis (CognitiveMirror) or serendipity discovery (SerendipityEngine) on blocks matched by an inner filter expression. It introduces an async execution path that bridges the synchronous SQL-based DSL with the async Rust analysis engines in `quilt-analysis`.

**Key characteristics:**
- Top-level only (no nesting inside other operators)
- Async execution (separate from sync `build_sql()` path)
- Filter-first: inner expression evaluated via SQL to get blocks, then blocks resolved to pages for analysis
- Returns typed JSON results (CognitiveMap or Vec<SerendipityConnection>)

## Grammar

```
analyze-expr     ::= "(analyze" inner-expr analysis-kind [keyword-args] ")"
inner-expr       ::= filter-expression
analysis-kind    ::= "cognitive_mirror" | "serendipity"
keyword-args     ::= { ":" keyword value }
keyword          ::= "limit" | "min-confidence" | "temporal-window-days"
value            ::= integer | float
```

## Operator Specification: analyze

### Syntax

| Variant | Full Syntax |
|---------|-------------|
| CognitiveMirror | `(analyze (filter) cognitive_mirror)` |
| Serendipity (defaults) | `(analyze (filter) serendipity)` |
| Serendipity (custom) | `(analyze (filter) serendipity :limit 20 :min-confidence 0.5)` |
| Serendipity (temporal) | `(analyze (filter) serendipity :temporal-window-days 30)` |

### Parsing Rules

1. **Operator recognition**: `(analyze ...)` is recognized in `parse_compound()` alongside other operators
2. **Two positional args required**:
   - Arg 0: inner filter expression (recursively parsed via `parse_expr()`)
   - Arg 1: analysis kind (`cognitive_mirror` or `serendipity`)
3. **Keyword args** (all optional):
   - `:limit N` — maximum connections to return (default: 20)
   - `:min-confidence F` — minimum confidence score 0.0–1.0 (default: 0.3)
   - `:temporal-window-days N` — look back N days (default: 30, None for page-scoped)
4. **Prohibited**: analyze cannot be nested inside other operators

### Semantic Interpretation

`analyze` evaluates the inner filter to get blocks via SQL → resolves blocks to their containing page(s) → calls the appropriate analysis engine.

### Execution Flow

```
DSL string
  → parse() [sync, in quilt-query]
  → QueryExpr::Analyze AST
  → QueryService::prepare_analyze() [sync, returns inner SQL + params]
  → Execute SQL [sync, gets blocks]
  → QueryService::execute_analyze() [NEW async]
    → CognitiveMirror::analyze_blocks()  OR
    → SerendipityEngine::find_connections()
  → AnalyzeResult JSON
```

## Return Types

### AnalyzeResult enum

```rust
pub enum AnalyzeResult {
    CognitiveMap(CognitiveMap),
    SerendipityConnections(Vec<SerendipityConnection>),
}
```

Where `CognitiveMap` and `SerendipityConnection` are imported from `quilt-analysis`:
- `CognitiveMap` contains `clusters`, `density`, `frontiers`, `gaps`, `influences`
- `SerendipityConnection` contains `idea_a`, `idea_b`, `bridge_concept`, `confidence`, `explanation`, `connection_type`

## API Changes

### QueryExpr::Analyze (parser.rs)

```rust
pub enum QueryExpr {
    // ... existing variants ...
    Analyze {
        inner: Box<QueryExpr>,
        kind: AnalyzeKind,
    },
}
```

### AnalyzeKind enum (parser.rs)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AnalyzeKind {
    CognitiveMirror,
    Serendipity {
        limit: Option<usize>,
        min_confidence: Option<f32>,
        temporal_window_days: Option<i64>,
    },
}
```

### AnalyzeResult enum (executor.rs)

```rust
use quilt_analysis::cognitive_mirror::types::CognitiveMap;
use quilt_analysis::serendipity::types::SerendipityConnection;

#[derive(Debug, Clone)]
pub enum AnalyzeResult {
    CognitiveMap(CognitiveMap),
    SerendipityConnections(Vec<SerendipityConnection>),
}
```

### AnalyzeError enum (executor.rs)

```rust
#[derive(Debug, Error)]
pub enum AnalyzeError {
    #[error("Analysis engine not configured: {0}")]
    EngineNotConfigured(String),
    #[error("Cognitive analysis failed: {0}")]
    CognitiveError(#[from] quilt_analysis::cognitive_mirror::engine::CognitiveError),
    #[error("Serendipity analysis failed: {0}")]
    SerendipityError(#[from] quilt_analysis::serendipity::engine::SerendipityError),
    #[error("Block repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
}
```

## Executor Changes

### execute_analyze() async method (executor.rs)

```rust
use quilt_analysis::{CognitiveMirror, SerendipityEngine};
use quilt_analysis::serendipity::types::SerendipityQuery;
use quilt_domain::repositories::BlockRepository;
use std::sync::Arc;

pub async fn execute_analyze(
    &self,
    expr: &QueryExpr,
    block_repo: Arc<dyn BlockRepository>,
    cognitive_mirror: Option<Arc<CognitiveMirror>>,
    serendipity_engine: Option<Arc<SerendipityEngine>>,
) -> Result<AnalyzeResult, AnalyzeError> {
    // 1. Extract inner filter and kind from QueryExpr::Analyze
    // 2. Build SQL for inner filter via build_sql()
    // 3. Execute SQL to get blocks
    // 4. For CognitiveMirror: call cognitive_mirror.analyze_blocks(blocks)
    // 5. For Serendipity: build SerendipityQuery, call serendipity_engine.find_connections()
    // 6. Return AnalyzeResult
}
```

### QueryService bridge (quilt-application/src/query_service.rs)

```rust
pub struct QueryService {
    parser: QueryParser,
    executor: QueryExecutor,
}

impl QueryService {
    /// Prepare an analyze expression for async execution.
    /// Returns (inner_sql, inner_params, analyze_kind) for later execute_analyze().
    pub fn prepare_analyze(&self, dsl: &str) -> Result<(String, Vec<String>, AnalyzeKind), String>;

    /// Execute an analyze expression against the analysis engines.
    pub async fn execute_analyze(
        &self,
        inner_sql: &str,
        inner_params: &[String],
        kind: &AnalyzeKind,
        block_repo: Arc<dyn BlockRepository>,
        cognitive_mirror: Option<Arc<CognitiveMirror>>,
        serendipity_engine: Option<Arc<SerendipityEngine>>,
    ) -> Result<AnalyzeResult, AnalyzeError>;
}
```

## MCP Integration

### quilt_query tool routing (quilt-mcp/src/server.rs)

The `quilt_query` tool handler detects `QueryExpr::Analyze` variants and routes to the async path:

```rust
async fn tool_quilt_query(&self, args: &serde_json::Value) -> Result<String, String> {
    let dsl = args.get("query").and_then(|v| v.as_str()).unwrap();
    let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(100);

    let parsed = self.parser.parse(dsl).map_err(|e| e.to_string())?;

    match parsed {
        QueryExpr::Analyze { inner, kind } => {
            // Route to async execute_analyze path
            let (sql, params) = self.query_service.prepare_analyze(dsl)?;
            let result = self.query_service.execute_analyze(
                &sql,
                &params,
                &kind,
                self.block_repo.clone(),
                self.cognitive_mirror.clone(),
                self.serendipity_engine.clone(),
            ).await?;
            Ok(serde_json::to_string_pretty(&result).unwrap())
        }
        _ => {
            // Existing sync path
            let result = self.query_service.prepare(dsl, limit)?;
            Ok(result.sql)
        }
    }
}
```

## Test Scenarios

### Parser tests

| Input | Expected AST |
|-------|--------------|
| `(analyze (task TODO) cognitive_mirror)` | `QueryExpr::Analyze { inner: Task(["TODO"]), kind: CognitiveMirror }` |
| `(analyze (page "X") serendipity)` | `QueryExpr::Analyze { inner: Page("X"), kind: Serendipity { limit: None, min_confidence: None, temporal_window_days: None } }` |
| `(analyze (task TODO) serendipity :limit 20)` | `QueryExpr::Analyze { inner: Task(["TODO"]), kind: Serendipity { limit: Some(20), ... } }` |
| `(analyze (task TODO) serendipity :min-confidence 0.5)` | `QueryExpr::Analyze { inner: Task(["TODO"]), kind: Serendipity { min_confidence: Some(0.5), ... } }` |
| `(analyze (task TODO) serendipity :temporal-window-days 30)` | `QueryExpr::Analyze { inner: Task(["TODO"]), kind: Serendipity { temporal_window_days: Some(30), ... } }` |
| `(analyze (task TODO) serendipity :limit 10 :min-confidence 0.4 :temporal-window-days 14)` | Full Serendipity with all args |

### Parser error cases

| Input | Expected Error |
|-------|----------------|
| `(analyze)` | `ParseError::Invalid("analyze requires inner expression and kind")` |
| `(analyze (task TODO))` | `ParseError::Invalid("analyze requires analysis kind (cognitive_mirror or serendipity)")` |
| `(analyze (task TODO) unknown_kind)` | `ParseError::Invalid("Unknown analysis kind: unknown_kind")` |
| `(analyze (task TODO) cognitive_mirror :limit)` | `ParseError::Invalid("limit requires a number")` |
| `(analyze (task TODO) serendipity :min-confidence)` | `ParseError::Invalid("min-confidence requires a float")` |
| `(analyze (task TODO) serendipity :temporal-window-days)` | `ParseError::Invalid("temporal-window-days requires an integer")` |

### Executor tests

| AST | Engine Called | Expected Behavior |
|-----|---------------|------------------|
| `Analyze { inner: Task, kind: CognitiveMirror }` | `CognitiveMirror::analyze_blocks()` | Blocks from SQL → analyze_blocks → CognitiveMap |
| `Analyze { inner: Page("X"), kind: Serendipity { limit: 20, ... } }` | `SerendipityEngine::find_connections()` | Blocks from SQL → SerendipityQuery → find_connections |

### Error cases

| Scenario | Expected Error |
|----------|----------------|
| CognitiveMirror not configured | `AnalyzeError::EngineNotConfigured("CognitiveMirror")` |
| SerendipityEngine not configured | `AnalyzeError::EngineNotConfigured("SerendipityEngine")` |
| SQL execution returns no blocks | Returns empty result (not an error) |
| Block repository unavailable | `AnalyzeError::Repository(...)` |
