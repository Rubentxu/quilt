# Design: DSL Aggregates (Change B1)

## Architecture

The three new operators (`aggregate`, `stats`, `group_by`) extend the existing query pipeline:

```
Query String → QueryParser.parse() → QueryExpr AST
                                            ↓
                           QueryExecutor.build_where() / build_sql()
                                            ↓
                                      SQL String
```

- **Parser** (`parser.rs`): `parse_compound()` dispatches on operator name. Three new arms (`"aggregate"`, `"stats"`, `"group_by"`) call `parse_aggregate()`, `parse_stats()`, `parse_group_by()` respectively. New AST nodes are `QueryExpr::Aggregate`, `QueryExpr::Stats`, `QueryExpr::GroupBy` with supporting enums `AggregateFn` and `StatsFn`.
- **Executor** (`executor.rs`): `build_where()` handles `QueryExpr::Aggregate` and `QueryExpr::GroupBy` by generating GROUP BY / DISTINCT subclauses. `build_sql()` detects aggregate variants and wraps the inner `build_where()` result as a subquery, or replaces the whole query shape for `stats`.
- **No new dependencies**: Uses SQLite's built-in aggregate functions (`COUNT`, `AVG`, `SUM`, `MIN`, `MAX`, `STDDEV_POP`, `VAR_POP`) and the `percentile_cont` window function.

## Module Changes

### quilt-query/src/parser.rs

1. **New enum `AggregateFn`** (after `QueryValue` impl block):
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum AggregateFn {
       Count,
       Avg,
       Sum,
       Min,
       Max,
   }
   ```

2. **New enum `StatsFn`**:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum StatsFn {
       Stddev,
       Variance,
       Median,
       Percentile(u8),
   }
   ```

3. **New enum `QueryError`** (replaces the `thiserror` derive on the existing `ParseError`; `ParseError` stays for parser-only errors, `QueryError` is the unified error type for the executor):
   ```rust
   use thiserror::Error;

   #[derive(Debug, Error)]
   pub enum QueryError {
       #[error("Syntax error: {0}")]
       Syntax(String),
       #[error("Invalid query: {0}")]
       Invalid(String),
       #[error("stats over array properties not supported")]
       ArrayPropertyNotSupported,
   }
   ```

4. **Three new `QueryExpr` variants** added to existing enum:
   ```rust
   Aggregate {
       inner: Box<QueryExpr>,
       group_by: String,
       aggregate_fn: AggregateFn,
   },
   Stats {
       property: String,
       compute: StatsFn,
   },
   GroupBy {
       inner: Box<QueryExpr>,
       property: String,
   },
   ```

5. **`parse_compound()` match arms** — add before the catch-all `_` arm:
   ```rust
   "aggregate" => self.parse_aggregate(rest),
   "stats"     => self.parse_stats(rest),
   "group_by"  => self.parse_group_by(rest),
   ```

6. **`parse_aggregate()` method**:
   ```rust
   fn parse_aggregate(&self, rest: &str) -> Result<QueryExpr, ParseError> {
       let args = self.split_args(rest);
       if args.len() != 3 {
           return Err(ParseError::Invalid(
               "aggregate requires (inner), (property ...), and (fn ...)".to_string()));
       }
       let inner = self.parse_expr(&args[0])?;
       let prop = Self::extract_property_arg(&args[1])?;
       let afn  = Self::parse_aggregate_fn(&args[2])?;
       Ok(QueryExpr::Aggregate {
           inner: Box::new(inner),
           group_by: prop,
           aggregate_fn: afn,
       })
   }

   fn extract_property_arg(s: &str) -> Result<String, ParseError> {
       let trimmed = s.trim();
       if !trimmed.starts_with("(property ") || !trimmed.ends_with(')') {
           return Err(ParseError::Invalid("expected (property <name>)".to_string()));
       }
       let inner = &trimmed[10..trimmed.len()-1];
       Ok(inner.trim().trim_matches('"').to_string())
   }

   fn parse_aggregate_fn(s: &str) -> Result<AggregateFn, ParseError> {
       let trimmed = s.trim();
       if !trimmed.starts_with("(fn ") || !trimmed.ends_with(')') {
           return Err(ParseError::Invalid("expected (fn <name>)".to_string()));
       }
       let inner = &trimmed[4..trimmed.len()-1];
       match inner.trim() {
           "count" => Ok(AggregateFn::Count),
           "avg"   => Ok(AggregateFn::Avg),
           "sum"   => Ok(AggregateFn::Sum),
           "min"   => Ok(AggregateFn::Min),
           "max"   => Ok(AggregateFn::Max),
           _ => Err(ParseError::Invalid(format!("unknown aggregate fn: {}", inner))),
       }
   }
   ```

7. **`parse_stats()` method**:
   ```rust
   fn parse_stats(&self, rest: &str) -> Result<QueryExpr, ParseError> {
       let args = self.split_args(rest);
       if args.len() != 2 {
           return Err(ParseError::Invalid(
               "stats requires (property ...) and (fn ...)".to_string()));
       }
       let prop = Self::extract_property_arg(&args[0])?;
       let sfn  = Self::parse_stats_fn(&args[1])?;
       Ok(QueryExpr::Stats { property: prop, compute: sfn })
   }

   fn parse_stats_fn(s: &str) -> Result<StatsFn, ParseError> {
       let trimmed = s.trim();
       if !trimmed.starts_with("(fn ") || !trimmed.ends_with(')') {
           return Err(ParseError::Invalid("expected (fn ...)".to_string()));
       }
       let inner = &trimmed[4..trimmed.len()-1];
       let parts: Vec<_> = inner.trim().split_whitespace().collect();
       match parts.as_slice() {
           ["stddev"]   => Ok(StatsFn::Stddev),
           ["variance"] => Ok(StatsFn::Variance),
           ["median"]   => Ok(StatsFn::Median),
           ["percentile", v] => {
               let n: u8 = v.parse()
                   .map_err(|_| ParseError::Invalid("percentile must be 0-100".to_string()))?;
               if n > 100 {
                   return Err(ParseError::Invalid("percentile must be 0-100".to_string()));
               }
               Ok(StatsFn::Percentile(n))
           }
           _ => Err(ParseError::Invalid(format!("unknown stats fn: {}", inner))),
       }
   }
   ```

8. **`parse_group_by()` method**:
   ```rust
   fn parse_group_by(&self, rest: &str) -> Result<QueryExpr, ParseError> {
       let args = self.split_args(rest);
       if args.len() != 2 {
           return Err(ParseError::Invalid(
               "group_by requires (inner) and (property ...)".to_string()));
       }
       let inner = self.parse_expr(&args[0])?;
       let prop  = Self::extract_property_arg(&args[1])?;
       Ok(QueryExpr::GroupBy { inner: Box::new(inner), property: prop })
   }
   ```

### quilt-query/src/executor.rs

1. **Update imports** to include new types:
   ```rust
   use crate::parser::{AggregateFn, QueryExpr, QueryValue, StatsFn, AggregateFn};
   ```

2. **`build_where()` new match arms** (before the catch-all `Tags` arm):
   ```rust
   QueryExpr::Aggregate { inner, group_by, aggregate_fn } => {
       let (inner_where, mut params) = self.build_where(inner);
       let prop_path = format!("json_extract(properties, '$.{}')", group_by);
       let null_check = format!("{} IS NOT NULL", prop_path);
       let where_clause = if inner_where.is_empty() {
           null_check.clone()
       } else {
           format!("{} AND {}", inner_where, null_check)
       };
       // Aggregate generates a GROUP BY — handled in build_sql
       (format!("{} AND {} GROUP BY {}", where_clause, prop_path, prop_path), params)
   }

   QueryExpr::Stats { property, compute } => {
       let prop_path = format!("json_extract(properties, '$.{}')", property);
       let null_check = format!("{} IS NOT NULL", prop_path);
       let fn_sql = match compute {
           StatsFn::Stddev    => format!("STDDEV_POP({})", prop_path),
           StatsFn::Variance => format!("VAR_POP({})", prop_path),
           StatsFn::Median    => {
               format!("percentile_cont(0.5) WITHIN GROUP (ORDER BY {})", prop_path)
           }
           StatsFn::Percentile(p) => {
               let frac = *p as f64 / 100.0;
               format!("percentile_cont({}) WITHIN GROUP (ORDER BY {})", frac, prop_path)
           }
       };
       (format!("{} AND {}", null_check, fn_sql), vec![])
   }

   QueryExpr::GroupBy { inner, property } => {
       let (inner_where, mut params) = self.build_where(inner);
       let prop_path = format!("json_extract(properties, '$.{}')", property);
       let null_check = format!("{} IS NOT NULL", prop_path);
       let where_clause = if inner_where.is_empty() {
           null_check.clone()
       } else {
           format!("{} AND {}", inner_where, null_check)
       };
       // DISTINCT GROUP BY handled in build_sql
       (format!("{} AND {} GROUP BY {}", where_clause, prop_path, prop_path), params)
   }
   ```

3. **`build_sql()` new match arms** — adds cases before the `SAMPLE` handling:
   ```rust
   QueryExpr::Aggregate { inner, group_by, aggregate_fn } => {
       let (inner_where, mut params) = self.build_where(inner);
       let prop_path = format!("json_extract(properties, '$.{}')", group_by);
       let fn_sql = match aggregate_fn {
           AggregateFn::Count => "COUNT(*)",
           AggregateFn::Avg  => "AVG(CAST(json_extract(properties, '$.count') AS REAL))",
           AggregateFn::Sum  => "SUM(CAST(json_extract(properties, '$.count') AS REAL))",
           AggregateFn::Min  => "MIN(CAST(json_extract(properties, '$.count') AS REAL))",
           AggregateFn::Max  => "MAX(CAST(json_extract(properties, '$.count') AS REAL))",
       };
       let null_check = format!("{} IS NOT NULL", prop_path);
       let where_clause = if inner_where.is_empty() {
           null_check.clone()
       } else {
           format!("{} AND {}", inner_where, null_check)
       };
       let sql = format!(
           "SELECT {}, {} \
            FROM blocks b \
            JOIN pages p ON b.page_id = p.id \
            WHERE {} \
            GROUP BY {}",
           prop_path, fn_sql, where_clause, prop_path
       );
       (sql, params)
   }

   QueryExpr::Stats { property, compute } => {
       let prop_path = format!("json_extract(properties, '$.{}')", property);
       let fn_sql = match compute {
           StatsFn::Stddev    => format!("STDDEV_POP({})", prop_path),
           StatsFn::Variance => format!("VAR_POP({})", prop_path),
           StatsFn::Median   => {
               format!("percentile_cont(0.5) WITHIN GROUP (ORDER BY {})", prop_path)
           }
           StatsFn::Percentile(p) => {
               let frac = *p as f64 / 100.0;
               format!("percentile_cont({}) WITHIN GROUP (ORDER BY {})", frac, prop_path)
           }
       };
       let sql = format!(
           "SELECT {} \
            FROM blocks b \
            JOIN pages p ON b.page_id = p.id \
            WHERE {} IS NOT NULL",
           fn_sql, prop_path
       );
       (sql, vec![])
   }

   QueryExpr::GroupBy { inner, property } => {
       let (inner_where, mut params) = self.build_where(inner);
       let prop_path = format!("json_extract(properties, '$.{}')", property);
       let null_check = format!("{} IS NOT NULL", prop_path);
       let where_clause = if inner_where.is_empty() {
           null_check.clone()
       } else {
           format!("{} AND {}", inner_where, null_check)
       };
       let sql = format!(
           "SELECT DISTINCT {} \
            FROM blocks b \
            JOIN pages p ON b.page_id = p.id \
            WHERE {}",
           prop_path, where_clause
       );
       (sql, params)
   }
   ```

### quilt-query/src/lib.rs

Update public exports:
```rust
pub use parser::{AggregateFn, QueryParser, QueryExpr, ParseError, StatsFn};
pub use executor::QueryExecutor;
pub use parser::QueryError;
```

## Implementation Details

### aggregate SQL generation

```sql
SELECT json_extract(properties, '$.author'), COUNT(*)
FROM blocks b
JOIN pages p ON b.page_id = p.id
WHERE marker IN (?) AND json_extract(properties, '$.author') IS NOT NULL
GROUP BY json_extract(properties, '$.author')
```

### stats SQL generation

**stddev:**
```sql
SELECT STDDEV_POP(json_extract(properties, '$.score'))
FROM blocks b
JOIN pages p ON b.page_id = p.id
WHERE json_extract(properties, '$.score') IS NOT NULL
```

**median:**
```sql
SELECT percentile_cont(0.5) WITHIN GROUP (ORDER BY json_extract(properties, '$.latency'))
FROM blocks b
JOIN pages p ON b.page_id = p.id
WHERE json_extract(properties, '$.latency') IS NOT NULL
```

**percentile N:**
```sql
SELECT percentile_cont(N/100.0) WITHIN GROUP (ORDER BY json_extract(properties, '$.p'))
FROM blocks b
JOIN pages p ON b.page_id = p.id
WHERE json_extract(properties, '$.p') IS NOT NULL
```

### group_by SQL generation

```sql
SELECT DISTINCT json_extract(properties, '$.author')
FROM blocks b
JOIN pages p ON b.page_id = p.id
WHERE marker IN (?) AND json_extract(properties, '$.author') IS NOT NULL
```

### Error handling flow

1. **Parser errors** (`ParseError::Syntax`, `ParseError::Invalid`) are returned directly from `QueryParser::parse()`.
2. **Array property detection** occurs in the executor when `build_sql()` encounters a `Stats` variant — the check `json_extract(properties, '$.prop')` returns a JSON array type in SQLite, which causes `STDDEV_POP`/`VAR_POP` to return NULL. A post-execution check is applied: if the result is NULL for a known non-array property, the error is not raised; if a property is known to be a JSON array (detected via `json_type()` returning `'array'`), `QueryError::ArrayPropertyNotSupported` is returned.
3. **Unified `Result` return**: `build_sql()` returns `(String, Vec<SqlParam>)` in the current design. Per the spec's open question #4 decision, the executor does not adopt `Result` in this change — error propagation from parser is sufficient; runtime errors like array properties surface as empty result sets until a future enhancement adds validation.

## File: crates/quilt-query/src/parser.rs — Changes

**Lines 9–18**: Replace `ParseError` definition with `QueryError` (adds `ArrayPropertyNotSupported`):
```rust
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Syntax error: {0}")]
    Syntax(String),
    #[error("Invalid query: {0}")]
    Invalid(String),
    #[error("stats over array properties not supported")]
    ArrayPropertyNotSupported,
}
```

**Lines 24–52**: Add three new `QueryExpr` variants after `Sample(usize)`.

**After line 79**: Add `AggregateFn` and `StatsFn` enums.

**Lines 178–191**: Add three new match arms to `parse_compound()`.

**After line 272**: Add `parse_aggregate()`, `parse_stats()`, `parse_group_by()` with their helpers.

## File: crates/quilt-query/src/executor.rs — Changes

**Lines 6**: Update import to include `AggregateFn` and `StatsFn`.

**Lines 72–179**: Add three new match arms to `build_where()` before the `Tags` arm.

**Lines 212–232**: Replace the `build_sql()` body with a switch that detects `Aggregate`, `Stats`, `GroupBy` first (generating full SQL) and falls back to the existing `WHERE`-based approach for other variants.

## Tests to add

### In `crates/quilt-query/src/parser.rs` (tests module)

```rust
#[test]
fn test_parse_aggregate() {
    let result = parse("(aggregate (task todo) (property author) (fn count))");
    assert_eq!(
        result,
        QueryExpr::Aggregate {
            inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
            group_by: "author".to_string(),
            aggregate_fn: AggregateFn::Count,
        }
    );
}

#[test]
fn test_parse_aggregate_avg() {
    let result = parse("(aggregate (priority a) (property priority) (fn avg))");
    assert_eq!(result, QueryExpr::Aggregate { ... });
}

#[test]
fn test_parse_stats_stddev() {
    let result = parse("(stats (property score) (fn stddev))");
    assert_eq!(
        result,
        QueryExpr::Stats { property: "score".to_string(), compute: StatsFn::Stddev }
    );
}

#[test]
fn test_parse_stats_median() {
    let result = parse("(stats (property latency) (fn median))");
    assert_eq!(
        result,
        QueryExpr::Stats { property: "latency".to_string(), compute: StatsFn::Median }
    );
}

#[test]
fn test_parse_stats_percentile() {
    let result = parse("(stats (property p) (fn percentile 95))");
    assert_eq!(
        result,
        QueryExpr::Stats { property: "p".to_string(), compute: StatsFn::Percentile(95) }
    );
}

#[test]
fn test_parse_stats_percentile_0() {
    let result = parse("(stats (property p) (fn percentile 0))");
    assert_eq!(result, QueryExpr::Stats { property: "p".to_string(), compute: StatsFn::Percentile(0) });
}

#[test]
fn test_parse_group_by() {
    let result = parse("(group_by (task todo) (property author))");
    assert_eq!(
        result,
        QueryExpr::GroupBy {
            inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
            property: "author".to_string(),
        }
    );
}

#[test]
fn test_parse_aggregate_missing_property() {
    let err = parse_err("(aggregate (task todo) (fn count))");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_aggregate_missing_fn() {
    let err = parse_err("(aggregate (task todo) (property author))");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_stats_percentile_oob() {
    let err = parse_err("(stats (property p) (fn percentile 150))");
    assert!(matches!(err, ParseError::Invalid(_)));
}

#[test]
fn test_parse_stats_unknown_fn() {
    let err = parse_err("(stats (property p) (fn unknown))");
    assert!(matches!(err, ParseError::Invalid(_)));
}
```

### In `crates/quilt-query/src/executor.rs` (tests module)

```rust
#[test]
fn test_aggregate_count_sql() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::Aggregate {
        inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
        group_by: "author".to_string(),
        aggregate_fn: AggregateFn::Count,
    };
    let (sql, params) = executor.build_sql(&expr, 100);
    assert!(sql.contains("GROUP BY json_extract"));
    assert!(sql.contains("COUNT(*)"));
    assert!(sql.contains("marker IN"));
}

#[test]
fn test_stats_stddev_sql() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::Stats { property: "score".to_string(), compute: StatsFn::Stddev };
    let (sql, params) = executor.build_sql(&expr, 100);
    assert!(sql.contains("STDDEV_POP"));
    assert!(sql.contains("$.score"));
}

#[test]
fn test_stats_median_sql() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::Stats { property: "latency".to_string(), compute: StatsFn::Median };
    let (sql, params) = executor.build_sql(&expr, 100);
    assert!(sql.contains("percentile_cont(0.5)"));
    assert!(sql.contains("WITHIN GROUP"));
}

#[test]
fn test_stats_percentile_sql() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::Stats { property: "p".to_string(), compute: StatsFn::Percentile(95) };
    let (sql, params) = executor.build_sql(&expr, 100);
    assert!(sql.contains("percentile_cont(0.95)"));
}

#[test]
fn test_group_by_sql() {
    let executor = QueryExecutor::new();
    let expr = QueryExpr::GroupBy {
        inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
        property: "author".to_string(),
    };
    let (sql, params) = executor.build_sql(&expr, 100);
    assert!(sql.contains("SELECT DISTINCT json_extract"));
    assert!(sql.contains("marker IN"));
    assert!(sql.contains("$.author"));
}
```

## Dependencies

No new dependencies. Uses existing:
- `thiserror` (already in `Cargo.toml`)
- SQLite aggregate functions: `COUNT`, `AVG`, `SUM`, `MIN`, `MAX`, `STDDEV_POP`, `VAR_POP`
- SQLite window function: `percentile_cont`
