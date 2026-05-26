# Design: DSL `analyze` Operator (Change B2)

## Architecture

The `analyze` operator introduces an **async execution path** that bridges the synchronous SQL-based DSL with the async Rust analysis engines in `quilt-analysis`:

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

Key architectural properties:
- **Top-level only**: `analyze` cannot be nested inside other operators — enforced in `parse_compound()`.
- **Async bridge**: `execute_analyze()` is an `async fn` that takes `Arc<dyn BlockRepository>` and optional engine handles.
- **Two-phase execution**: `prepare_analyze()` (sync) returns inner SQL + `AnalyzeKind`; `execute_analyze()` (async) consumes those and runs the engine.
- **Typed results**: `AnalyzeResult` enum wraps `CognitiveMap` or `Vec<SerendipityConnection>` from `quilt-analysis`.

## Module Changes

### quilt-query/src/parser.rs

**1. New enum `AnalyzeKind`** (after `StatsFn` definition, around line 121):

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

**2. New `QueryExpr::Analyze` variant** (added to the `QueryExpr` enum after `GroupBy`):

```rust
Analyze {
    inner: Box<QueryExpr>,
    kind: AnalyzeKind,
},
```

**3. New match arm in `parse_compound()`** (add before the catch-all `_` arm, around line 243):

```rust
"analyze" => self.parse_analyze(rest),
```

**4. New `parse_analyze()` method** (add after `parse_group_by()`, around line 436):

```rust
fn parse_analyze(&self, rest: &str) -> Result<QueryExpr, ParseError> {
    let args = self.split_args(rest);
    if args.len() < 2 {
        return Err(ParseError::Invalid(
            "analyze requires inner expression and kind".to_string(),
        ));
    }

    let inner = self.parse_expr(&args[0])?;
    let kind = self.parse_analyze_kind(&args[1], &args[2..])?;

    Ok(QueryExpr::Analyze {
        inner: Box::new(inner),
        kind,
    })
}

fn parse_analyze_kind(&self, kind_str: &str, rest: &[String]) -> Result<AnalyzeKind, ParseError> {
    match kind_str {
        "cognitive_mirror" => {
            if !rest.is_empty() {
                return Err(ParseError::Invalid(
                    "cognitive_mirror takes no keyword arguments".to_string(),
                ));
            }
            Ok(AnalyzeKind::CognitiveMirror)
        }
        "serendipity" => {
            let mut limit = None;
            let mut min_confidence = None;
            let mut temporal_window_days = None;

            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    ":limit" => {
                        i += 1;
                        if i >= rest.len() {
                            return Err(ParseError::Invalid("limit requires a number".to_string()));
                        }
                        limit = Some(
                            rest[i].parse()
                                .map_err(|_| ParseError::Invalid("limit requires a number".to_string()))?
                        );
                    }
                    ":min-confidence" => {
                        i += 1;
                        if i >= rest.len() {
                            return Err(ParseError::Invalid("min-confidence requires a float".to_string()));
                        }
                        min_confidence = Some(
                            rest[i].parse()
                                .map_err(|_| ParseError::Invalid("min-confidence requires a float".to_string()))?
                        );
                    }
                    ":temporal-window-days" => {
                        i += 1;
                        if i >= rest.len() {
                            return Err(ParseError::Invalid("temporal-window-days requires an integer".to_string()));
                        }
                        temporal_window_days = Some(
                            rest[i].parse()
                                .map_err(|_| ParseError::Invalid("temporal-window-days requires an integer".to_string()))?
                        );
                    }
                    _ => {
                        return Err(ParseError::Invalid(format!(
                            "Unknown keyword in analyze: {}", rest[i]
                        )));
                    }
                }
                i += 1;
            }

            Ok(AnalyzeKind::Serendipity {
                limit,
                min_confidence,
                temporal_window_days,
            })
        }
        _ => Err(ParseError::Invalid(format!(
            "Unknown analysis kind: {}", kind_str
        ))),
    }
}
```

### quilt-query/src/executor.rs

**1. Update imports** to include `AnalyzeKind` and new result types:

```rust
use crate::parser::{AggregateFn, AnalyzeKind, QueryExpr, QueryValue, StatsFn};
```

**2. Add `AnalyzeResult` enum** (after `SqlParam` impl block, around line 31):

```rust
#[derive(Debug, Clone)]
pub enum AnalyzeResult {
    CognitiveMap(quilt_analysis::cognitive_mirror::types::CognitiveMap),
    SerendipityConnections(Vec<quilt_analysis::serendipity::types::SerendipityConnection>),
}
```

**3. Add `AnalyzeError` enum** (after `AnalyzeResult`, around line 39):

```rust
#[derive(Debug, thiserror::Error)]
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

**4. Add `build_analyze_sql()` method to `QueryExecutor`** (add after `value_to_param()`, around line 362):

```rust
pub fn build_analyze_sql(&self, expr: &QueryExpr) -> Result<(String, Vec<SqlParam>), ParseError> {
    match expr {
        QueryExpr::Analyze { inner, .. } => {
            let (where_clause, params) = self.build_where(inner);
            let sql = if where_clause.is_empty() {
                format!(
                    "SELECT b.* FROM blocks b JOIN pages p ON b.page_id = p.id LIMIT {}",
                    1000 // hard cap for analyze
                )
            } else {
                format!(
                    "SELECT b.*, p.name as page_name \
                     FROM blocks b \
                     JOIN pages p ON b.page_id = p.id \
                     WHERE {} \
                     LIMIT {}",
                    where_clause, 1000
                )
            };
            Ok((sql, params))
        }
        _ => Err(ParseError::Invalid("Expected Analyze expression".to_string())),
    }
}
```

### quilt-query/src/lib.rs

Update public exports:

```rust
pub use parser::{AggregateFn, AnalyzeKind, QueryParser, QueryExpr, ParseError, StatsFn, QueryError};
pub use executor::{AnalyzeError, AnalyzeResult, QueryExecutor, SqlParam};
```

### quilt-application/src/query_service.rs

**1. Add imports** at top of file:

```rust
use quilt_analysis::{CognitiveMirror, SerendipityEngine};
use quilt_analysis::serendipity::types::SerendipityQuery;
use quilt_domain::repositories::BlockRepository;
use quilt_query::{AnalyzeKind, AnalyzeResult, QueryExpr};
use std::sync::Arc;
```

**2. Add `prepare_analyze()` method to `QueryService`** (add after `prepare()`, around line 128):

```rust
pub fn prepare_analyze(&self, dsl: &str) -> Result<(String, Vec<String>, AnalyzeKind), String> {
    let ast = self
        .parser
        .parse(dsl)
        .map_err(|e| format!("Parse error: {}", e))?;

    match ast {
        QueryExpr::Analyze { inner, kind } => {
            let (sql, params) = self.executor.build_analyze_sql(&ast)
                .map_err(|e| format!("Analyze SQL error: {}", e))?;
            Ok((sql, params.iter().map(|p| p.as_string()).collect(), kind))
        }
        _ => Err("prepare_analyze requires an Analyze expression".to_string()),
    }
}
```

**3. Add `execute_analyze()` method to `QueryService`** (add after `prepare_analyze()`, around line 144):

```rust
pub async fn execute_analyze(
    &self,
    inner_sql: &str,
    inner_params: &[String],
    kind: &AnalyzeKind,
    block_repo: Arc<dyn BlockRepository>,
    cognitive_mirror: Option<Arc<CognitiveMirror>>,
    serendipity_engine: Option<Arc<SerendipityEngine>>,
) -> Result<AnalyzeResult, quilt_query::AnalyzeError> {
    let pool = self.executor.db_pool();

    let blocks: Vec<quilt_domain::entities::Block> = {
        let mut conn = pool.acquire().await
            .map_err(|e| quilt_query::AnalyzeError::Repository(
                quilt_domain::errors::DomainError::Internal(e.to_string())
            ))?;

        let rows: Vec<(String, String, i64, String)> = sqlx::query_as(
            &format!("SELECT id, content, page_id, properties FROM blocks WHERE id IN (SELECT id FROM ({}) AS inner_q)", inner_sql)
        )
        .bind_all(inner_params.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice())
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| quilt_query::AnalyzeError::Repository(
            quilt_domain::errors::DomainError::Internal(e.to_string())
        ))?;

        rows.into_iter().map(|(id, content, page_id, properties)| {
            quilt_domain::entities::Block {
                id: quilt_domain::value_objects::Uuid::from_string(&id),
                page_id: quilt_domain::value_objects::Uuid::from_string(&page_id),
                content,
                properties: serde_json::from_str(&properties).unwrap_or_default(),
                ..Default::default()
            }
        }).collect()
    };

    match kind {
        AnalyzeKind::CognitiveMirror => {
            let mirror = cognitive_mirror
                .ok_or_else(|| quilt_query::AnalyzeError::EngineNotConfigured("CognitiveMirror".to_string()))?;
            let result = mirror.analyze_blocks(&blocks).await;
            Ok(AnalyzeResult::CognitiveMap(result))
        }
        AnalyzeKind::Serendipity { limit, min_confidence, temporal_window_days } => {
            let engine = serendipity_engine
                .ok_or_else(|| quilt_query::AnalyzeError::EngineNotConfigured("SerendipityEngine".to_string()))?;

            let query = SerendipityQuery {
                topic: None,
                limit: limit.unwrap_or(20),
                offset: 0,
                min_confidence: min_confidence.unwrap_or(0.3),
                temporal_window_days: *temporal_window_days,
                page_id: None,
            };

            let result = engine.find_connections(query).await?;
            Ok(AnalyzeResult::SerendipityConnections(result))
        }
    }
}
```

### quilt-mcp/src/server.rs

**1. Update `tool_quilt_query()` routing** (around line 602):

```rust
"quilt_query" => {
    let dsl = params.arguments.get("dsl")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'dsl' argument")?;
    let limit = params.arguments.get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(100);

    let parsed = self.parser.parse(dsl).map_err(|e| e.to_string())?;

    match parsed {
        QueryExpr::Analyze { inner: _, kind } => {
            let (sql, params, analyze_kind) = self.query_service.prepare_analyze(dsl)
                .map_err(|e| e.to_string())?;
            let result = self.query_service.execute_analyze(
                &sql,
                &params,
                &analyze_kind,
                self.block_repo.clone(),
                self.cognitive_mirror.clone(),
                self.serendipity_engine.clone(),
            ).await.map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&result).unwrap())
        }
        _ => {
            let result = self.query_service.prepare(dsl, limit)
                .map_err(|e| e.to_string())?;
            Ok(result.sql)
        }
    }
}
```

## Async Execution Flow

Step-by-step flow for `(analyze (task TODO) serendipity :limit 20)`:

1. **MCP tool call** → `tool_quilt_query()` receives `dsl: "(analyze (task TODO) serendipity :limit 20)"`
2. **Parse** → `QueryParser::parse()` returns `QueryExpr::Analyze { inner: Task(["TODO"]), kind: Serendipity { limit: Some(20), ... } }`
3. **Route** → `match` on `QueryExpr::Analyze` goes to async path
4. **Prepare** → `QueryService::prepare_analyze()` calls `executor.build_analyze_sql()` which:
   - Extracts inner `Task` expression
   - Calls `build_where(Task(["TODO"]))` → `("marker IN (?)", [SqlParam::String("todo")])`
   - Returns full SQL with JOIN and LIMIT 1000
5. **SQL Execute** → `execute_analyze()` acquires a DB connection, runs the SQL, fetches block rows
6. **Block resolution** → Rows mapped to `Vec<Block>` entities
7. **Engine call** → `SerendipityEngine::find_connections(SerendipityQuery { limit: 20, ... })`
8. **Result** → `AnalyzeResult::SerendipityConnections(Vec<SerendipityConnection>)` serialized to JSON

## Error Handling

| Error source | Error type | Propagation |
|---|---|---|
| `analyze` nested inside another operator | `ParseError::Invalid` | Thrown by parser |
| Unknown `analysis-kind` | `ParseError::Invalid` | Thrown by `parse_analyze_kind()` |
| Keyword arg type mismatch (`:limit` with float) | `ParseError::Invalid` | Thrown by `parse_analyze_kind()` |
| CognitiveMirror not configured in server | `AnalyzeError::EngineNotConfigured` | Returned by `execute_analyze()` |
| SerendipityEngine not configured in server | `AnalyzeError::EngineNotConfigured` | Returned by `execute_analyze()` |
| DB pool unavailable | `AnalyzeError::Repository(DomainError::Internal)` | Wrapped in `execute_analyze()` |
| `CognitiveMirror::analyze_blocks()` fails | `AnalyzeError::CognitiveError` | `#[from]` propagation |
| `SerendipityEngine::find_connections()` fails | `AnalyzeError::SerendipityError` | `#[from]` propagation |
| SQL execution fails | `AnalyzeError::Repository` | Wrapped DomainError |

## Tests to Add

### In `crates/quilt-query/src/parser.rs` (tests module)

```rust
#[test]
fn test_parse_analyze_cognitive_mirror() {
    let result = parse("(analyze (task todo) cognitive_mirror)");
    assert_eq!(
        result,
        QueryExpr::Analyze {
            inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
            kind: AnalyzeKind::CognitiveMirror,
        }
    );
}

#[test]
fn test_parse_analyze_serendipity_defaults() {
    let result = parse("(analyze (page \"X\") serendipity)");
    assert_eq!(
        result,
        QueryExpr::Analyze {
            inner: Box::new(QueryExpr::Page("X".to_string())),
            kind: AnalyzeKind::Serendipity {
                limit: None,
                min_confidence: None,
                temporal_window_days: None,
            },
        }
    );
}

#[test]
fn test_parse_analyze_serendipity_with_limit() {
    let result = parse("(analyze (task todo) serendipity :limit 20)");
    match result {
        QueryExpr::Analyze { kind: AnalyzeKind::Serendipity { limit, .. }, .. } => {
            assert_eq!(limit, Some(20));
        }
        _ => panic!("expected Serendipity with limit"),
    }
}

#[test]
fn test_parse_analyze_serendipity_full() {
    let result = parse("(analyze (task todo) serendipity :limit 10 :min-confidence 0.4 :temporal-window-days 14)");
    match result {
        QueryExpr::Analyze {
            kind: AnalyzeKind::Serendipity { limit, min_confidence, temporal_window_days },
            ..
        } => {
            assert_eq!(limit, Some(10));
            assert_eq!(min_confidence, Some(0.4));
            assert_eq!(temporal_window_days, Some(14));
        }
        _ => panic!("expected full Serendipity"),
    }
}

#[test]
fn test_parse_analyze_empty() {
    let err = parse_err("(analyze)");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_analyze_missing_kind() {
    let err = parse_err("(analyze (task todo))");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_analyze_unknown_kind() {
    let err = parse_err("(analyze (task todo) unknown_kind)");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_analyze_limit_no_value() {
    let err = parse_err("(analyze (task todo) serendipity :limit)");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_analyze_min_confidence_no_value() {
    let err = parse_err("(analyze (task todo) serendipity :min-confidence)");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_analyze_temporal_window_no_value() {
    let err = parse_err("(analyze (task todo) serendipity :temporal-window-days)");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_analyze_cognitive_mirror_with_kwargs() {
    let err = parse_err("(analyze (task todo) cognitive_mirror :limit 5)");
    assert!(matches!(err, ParseError::Invalid(_)));
}
```

### In `crates/quilt-query/src/executor.rs` (tests module)

```rust
#[test]
fn test_build_analyze_sql_simple() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::Analyze {
        inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
        kind: AnalyzeKind::CognitiveMirror,
    };
    let (sql, params) = executor.build_analyze_sql(&expr).unwrap();
    assert!(sql.contains("SELECT b.*"));
    assert!(sql.contains("FROM blocks b"));
    assert!(sql.contains("JOIN pages p"));
    assert!(sql.contains("marker IN"));
    assert!(sql.contains("LIMIT 1000"));
}

#[test]
fn test_build_analyze_sql_page_filter() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::Analyze {
        inner: Box::new(QueryExpr::Page("Test".to_string())),
        kind: AnalyzeKind::Serendipity { limit: None, min_confidence: None, temporal_window_days: None },
    };
    let (sql, params) = executor.build_analyze_sql(&expr).unwrap();
    assert!(sql.contains("EXISTS"));
}

#[test]
fn test_build_analyze_sql_non_analyze_error() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::Task(vec!["todo".to_string()]);
    let result = executor.build_analyze_sql(&expr);
    assert!(result.is_err());
}
```

### In `crates/quilt-application/src/query_service.rs` (tests module)

```rust
#[tokio::test]
async fn test_prepare_analyze_round_trip() {
    let service = QueryService::new();
    let result = service.prepare_analyze("(analyze (task todo) cognitive_mirror)");
    assert!(result.is_ok());
    let (sql, params, kind) = result.unwrap();
    assert!(sql.contains("marker IN"));
    assert_eq!(kind, AnalyzeKind::CognitiveMirror);
}

#[tokio::test]
async fn test_prepare_analyze_non_analyze_error() {
    let service = QueryService::new();
    let result = service.prepare_analyze("(task todo)");
    assert!(result.is_err());
}
```

## Dependencies

**New dependencies** (add to `crates/quilt-query/Cargo.toml`):

```toml
quilt-analysis = { path = "../quilt-analysis" }
quilt-domain = { path = "../quilt-domain" }
```

These are already transitive dependencies of `quilt-application`, so no new external crates are introduced.

**Existing dependencies used**:
- `thiserror` (already in `Cargo.toml`) — for `AnalyzeError`
- `quilt_analysis::cognitive_mirror::types::CognitiveMap` — result type
- `quilt_analysis::serendipity::types::SerendipityQuery` — serendipity query builder
- `quilt_domain::entities::Block` — block representation
- `quilt_domain::repositories::BlockRepository` — trait for block access
