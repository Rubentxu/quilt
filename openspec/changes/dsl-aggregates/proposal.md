# Proposal: DSL Aggregates (Change B1)

## Executive Summary
This change extends the Quilt query DSL with three new operators—`aggregate`, `stats`, and `group_by`—enabling grouped analytics directly in query expressions. The parser adds three new `QueryExpr` variants, the executor generates appropriate SQL (GROUP BY with aggregate functions, statistical window functions), and the existing MCP `quilt_query` tool becomes the integration point.

## Intent

We are building analytical query capabilities into the DSL to support use cases like "count tasks by author", "compute average priority per project", or "find median response time". Currently the DSL only supports filtering and simple property lookups. Adding these three operators closes the functional gap between raw SQL and the DSL, allowing agents and users to express aggregation intent declaratively without writing SQL.

## Scope

### In scope
- **Parser** (`quilt-query/src/parser.rs`): Add `QueryExpr::Aggregate`, `QueryExpr::Stats`, `QueryExpr::GroupBy` variants. Add `AggregateFn` enum (Count, Avg, Sum, Min, Max) and `StatsFn` enum (Stddev, Variance, Median, Percentile(u8)). Wire parsing of new s-expressions.
- **Executor** (`quilt-query/src/executor.rs`): `build_sql()` handles all three new variants. `build_where()` is unaffected.
- **SQL generation**: GROUP BY with `json_extract` for property paths; SQLite aggregate functions; STDDEV_POP/VAR_POP via `sqlite3` extension; percentile via window `percentile_cont`.
- **Tests**: Unit tests for parser (round-trip parsing of new operators) and executor (SQL output verification). Integration via `quilt_query` MCP tool.
- **MCP**: Document that operators work via existing `quilt_query` tool; no new tool needed.

### Out of scope
- `analyze` operator (Change B2 — async, separate SDD)
- Nesting of superconjunto operators
- Full DSL grammar rewrite to a proper parser generator (deferred to future work)
- Persistence of intermediate results

## Approach

### Step 1: Parser changes
1. Add `AggregateFn` enum with variants: `Count`, `Avg`, `Sum`, `Min`, `Max`.
2. Add `StatsFn` enum with variants: `Stddev`, `Variance`, `Median`, `Percentile(u8)`.
3. Add `QueryExpr::Aggregate { inner: Box<QueryExpr>, group_by: String, aggregate_fn: AggregateFn }`.
4. Add `QueryExpr::Stats { property: String, compute: StatsFn }`.
5. Add `QueryExpr::GroupBy { inner: Box<QueryExpr>, property: String }`.
6. Add parsing rules in `parse_query_expr` for `aggregate`, `stats`, `group_by` s-expressions.

### Step 2: Executor changes
1. Extend `build_sql()` match arms for `QueryExpr::Aggregate`, `QueryExpr::Stats`, `QueryExpr::GroupBy`.
2. `Aggregate`: generate `SELECT json_extract(props, '$.group_by'), aggregate_fn(*) ... GROUP BY json_extract(props, '$.group_by')`.
3. `Stats`: generate statistical function call over property (stddev_pop, var_pop, percentile_cont).
4. `GroupBy`: generate `SELECT DISTINCT json_extract(props, '$.property') ...` (no aggregation).
5. Wrap existing `build_where()` result as subquery when needed.

### Step 3: Tests
1. Parser round-trip tests: parse `(aggregate (task TODO) (property author) (fn count))` and verify reconstructed string matches.
2. Executor tests: call `build_sql()` with new variants and assert on generated SQL string.
3. MCP integration test: invoke `quilt_query` with aggregate/stats/group_by expressions and verify results.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-------------|--------|------------|
| SQLite percentile window function syntax differs across versions | Medium | Medium | Use `percentile_cont(0.5) WITHIN GROUP (ORDER BY val)` for median; gate percentiles behind version check |
| `json_extract` performance on large tables | Medium | Low | Document column index requirement on `props`; suggest covering indexes for high-frequency queries |
| Aggregate fns may conflict with future `analyze` async semantics | Low | Medium | Keep `aggregate`/`stats`/`group_by` synchronous; `analyze` gets separate `QueryExpr` variant |
| Parser ambiguity between property names and function names | Low | Low | Enforce `(property name)` and `(fn count)` syntactic markers in parser |

## Open Questions

1. **Percentile representation**: `Percentile(u8)` stores a raw percentile value (e.g., 95 for 95th percentile). Should the parser accept float values (e.g., 0.95) or only integer 0–100? **Decision needed**: integer 0–100.
2. **Null handling**: When a property is null/missing in `json_extract`, should aggregates skip the row or count it as a distinct group? **Decision needed**: skip (SQL standard behavior via `WHERE property IS NOT NULL` or `GROUP BY` filtering).
3. **Stats over scalar vs. array properties**: If a property stores a JSON array (e.g., tags), should `stats` auto-unnest or reject? **Decision needed**: reject with a clear error; unnesting is out of scope.
4. **Executor error propagation**: Should `build_sql()` return `Result<String, QueryError>` for all variants or only for the new three? **Decision needed**: unify to `Result` for consistency.

## Dependencies

- Rust 2024 edition toolchain
- `quilt-query` crate (parser + executor modules)
- SQLite with FTS5 (already in use; no new extensions required for core aggregates)
- Existing `quilt_query` MCP tool (no changes to MCP protocol)

## Success Criteria

1. **Parser**: All three new operators parse without error from valid s-expressions.
2. **Executor**: `build_sql()` returns correct SQL for each operator (verified by test assertions).
3. **Round-trip**: Parsing a formatted expression and reprinting it produces an equivalent expression.
4. **MCP**: `quilt_query` tool accepts aggregate/stats/group_by expressions and returns results.
5. **Tests pass**: `cargo test -p quilt-query` runs green.
6. **No clippy warnings**: `cargo clippy -p quilt-query` reports zero warnings.
7. **Lints pass**: `cargo fmt --check` passes.
