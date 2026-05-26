# Spec: DSL Aggregates (Change B1)

## Overview

This change adds three new aggregate operators to the Quilt query DSL: `aggregate`, `stats`, and `group_by`. These operators enable grouped analytics directly in query expressions, supporting use cases like "count tasks by author", "compute average priority per project", and "find median response time".

## Grammar

```ebnf
<query-expr> ::= <aggregate-expr> | <stats-expr> | <group-by-expr> | <existing-expr>

<aggregate-expr> ::= "(aggregate" <inner-expr> "(property" <string> ")" "(fn" <aggregate-fn> ")" ")"
<aggregate-fn> ::= "count" | "avg" | "sum" | "min" | "max"

<stats-expr> ::= "(stats" "(property" <string> ")" "(fn" <stats-fn> ")" ")"
<stats-fn> ::= "stddev" | "variance" | "median" | "percentile" <u8>

<group-by-expr> ::= "(group_by" <inner-expr> "(property" <string> ")" ")"

<inner-expr> ::= <existing-expr>
```

## Operator Specifications

### aggregate

**Syntax**: `(aggregate (inner_expr) (property author) (fn count))`

**Parsing rules**:
- `inner_expr`: any valid QueryExpr parsed recursively
- `property`: string literal (property key to group by)
- `fn`: one of `count`, `avg`, `sum`, `min`, `max`

**SQL generation**:
```sql
SELECT json_extract(props, '$.author'), COUNT(*)
FROM blocks b
WHERE <inner_where> AND json_extract(props, '$.author') IS NOT NULL
GROUP BY json_extract(props, '$.author')
```

**NULL handling**: Rows where the property is null are NOT included in any group. The `WHERE` clause includes `AND json_extract(props, '$.property') IS NOT NULL`.

**Error cases**:
- Missing `property` or `fn` clause → `ParseError::Invalid`
- Unknown aggregate function → `ParseError::Invalid`

---

### stats

**Syntax**: `(stats (property count) (fn stddev))`

**Parsing rules**:
- `property`: string literal (numeric property to compute stats on)
- `fn`: one of `stddev`, `variance`, `median`, `percentile <u8>`

**SQL generation**:
| Function | SQL |
|----------|-----|
| `stddev` | `STDDEV_POP(json_extract(props, '$.count'))` |
| `variance` | `VAR_POP(json_extract(props, '$.count'))` |
| `median` | `percentile_cont(0.5) WITHIN GROUP (ORDER BY json_extract(props, '$.count'))` |
| `percentile N` | `percentile_cont(N/100.0) WITHIN GROUP (ORDER BY json_extract(props, '$.count'))` |

**Percentile conversion**: `u8` 0–100 → `f64` 0.0–1.0 by dividing by 100.0.

**NULL handling**: SQL standard — null values are skipped via SQL aggregate behavior.

**Error cases**:
- Array property detected → `QueryError::ArrayPropertyNotSupported`
- Unknown stats function → `ParseError::Invalid`
- Percentile value out of 0–100 range → `ParseError::Invalid`

---

### group_by

**Syntax**: `(group_by (inner_expr) (property author))`

**Parsing rules**:
- `inner_expr`: any valid QueryExpr parsed recursively
- `property`: string literal (property key to group by)

**SQL generation**:
```sql
SELECT DISTINCT json_extract(props, '$.author')
FROM blocks b
WHERE <inner_where> AND json_extract(props, '$.author') IS NOT NULL
```

**NULL handling**: Rows where property is null are NOT included (explicit `WHERE property IS NOT NULL`).

**Error cases**:
- Missing `property` clause → `ParseError::Invalid`

---

## API Changes

### QueryExpr enum

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum QueryExpr {
    // ... existing variants ...

    /// Aggregate with GROUP BY
    Aggregate {
        inner: Box<QueryExpr>,
        group_by: String,
        aggregate_fn: AggregateFn,
    },
    /// Statistical computation over a property
    Stats {
        property: String,
        compute: StatsFn,
    },
    /// Group by property (no aggregation)
    GroupBy {
        inner: Box<QueryExpr>,
        property: String,
    },
}
```

### AggregateFn enum

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

### StatsFn enum

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum StatsFn {
    Stddev,
    Variance,
    Median,
    Percentile(u8),  // 0-100
}
```

### QueryError enum (new)

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

---

## Error Handling

| Operator | Error Condition | Error Type |
|----------|-----------------|------------|
| `aggregate` | Missing `(property ...)` | `ParseError::Invalid` |
| `aggregate` | Missing `(fn ...)` | `ParseError::Invalid` |
| `aggregate` | Unknown function name | `ParseError::Invalid` |
| `stats` | Array property detected | `QueryError::ArrayPropertyNotSupported` |
| `stats` | Percentile out of 0–100 | `ParseError::Invalid` |
| `group_by` | Missing `(property ...)` | `ParseError::Invalid` |

The executor's `build_sql()` returns `Result<String, QueryError>` for consistency.

---

## Test Scenarios

### Parser tests

| Input | Expected AST |
|-------|--------------|
| `(aggregate (task todo) (property author) (fn count))` | `QueryExpr::Aggregate { inner: Task(["todo"]), group_by: "author", aggregate_fn: Count }` |
| `(aggregate (priority a) (property priority) (fn avg))` | `QueryExpr::Aggregate { inner: Priority(["a"]), group_by: "priority", aggregate_fn: Avg }` |
| `(stats (property score) (fn stddev))` | `QueryExpr::Stats { property: "score", compute: Stddev }` |
| `(stats (property latency) (fn median))` | `QueryExpr::Stats { property: "latency", compute: Median }` |
| `(stats (property p) (fn percentile 95))` | `QueryExpr::Stats { property: "p", compute: Percentile(95) }` |
| `(stats (property p) (fn percentile 0))` | `QueryExpr::Stats { property: "p", compute: Percentile(0) }` |
| `(group_by (task todo) (property author))` | `QueryExpr::GroupBy { inner: Task(["todo"]), property: "author" }` |

### Executor tests

| AST | Expected SQL (simplified) |
|-----|--------------------------|
| `Aggregate { inner: Task(["todo"]), group_by: "author", aggregate_fn: Count }` | `SELECT json_extract(props, '$.author'), COUNT(*) FROM blocks b WHERE marker IN (?) AND json_extract(props, '$.author') IS NOT NULL GROUP BY json_extract(props, '$.author')` |
| `Stats { property: "score", compute: Stddev }` | `SELECT STDDEV_POP(json_extract(props, '$.score')) FROM blocks b WHERE json_extract(props, '$.score') IS NOT NULL` |
| `Stats { property: "latency", compute: Median }` | `SELECT percentile_cont(0.5) WITHIN GROUP (ORDER BY json_extract(props, '$.latency')) FROM blocks b WHERE json_extract(props, '$.latency') IS NOT NULL` |
| `Stats { property: "p", compute: Percentile(95) }` | `SELECT percentile_cont(0.95) WITHIN GROUP (ORDER BY json_extract(props, '$.p')) FROM blocks b WHERE json_extract(props, '$.p') IS NOT NULL` |
| `GroupBy { inner: Task(["todo"]), property: "author" }` | `SELECT DISTINCT json_extract(props, '$.author') FROM blocks b WHERE marker IN (?) AND json_extract(props, '$.author') IS NOT NULL` |

### Error cases

| Input | Expected Error |
|-------|----------------|
| `(stats (property tags) (fn stddev))` where tags is array | `QueryError::ArrayPropertyNotSupported` |
| `(aggregate (task todo) (fn count))` — missing property | `ParseError::Invalid("aggregate requires (property ...)")` |
| `(stats (property x) (fn percentile 150))` | `ParseError::Invalid("percentile must be 0-100")` |
| `(aggregate (task todo) (property author))` — missing fn | `ParseError::Invalid("aggregate requires (fn ...)")` |

---

## Implementation Notes

1. **Parser**: Add parsing methods `parse_aggregate()`, `parse_stats()`, `parse_group_by()` in `parser.rs`
2. **Executor**: Extend `build_where()` and `build_sql()` in `executor.rs` to handle new variants, returning `Result<String, QueryError>`
3. **Array detection**: Check if property value is a JSON array at query planning time; return `QueryError::ArrayPropertyNotSupported`
4. **Percentile bounds**: Validate u8 0–100 in parser; convert to f64 for SQL with `value as f64 / 100.0`
