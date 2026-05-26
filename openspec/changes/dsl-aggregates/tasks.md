# Tasks: DSL Aggregates (Change B1)

## Task List

### Phase 1: Parser
- [x] 1.1 Add AggregateFn enum to parser.rs
- [x] 1.2 Add StatsFn enum to parser.rs
- [x] 1.3 Add QueryError enum to parser.rs
- [x] 1.4 Add Aggregate variant to QueryExpr enum
- [x] 1.5 Add Stats variant to QueryExpr enum
- [x] 1.6 Add GroupBy variant to QueryExpr enum
- [x] 1.7 Add parse_aggregate() method to QueryParser
- [x] 1.8 Add parse_stats() method to QueryParser
- [x] 1.9 Add parse_group_by() method to QueryParser
- [x] 1.10 Add helper: extract_property_arg()
- [x] 1.11 Add helper: parse_aggregate_fn()
- [x] 1.12 Add helper: parse_stats_fn()
- [x] 1.13 Wire new operators in parse_compound() match

### Phase 2: Executor
- [x] 2.1 Add build_sql() match arm for Aggregate
- [x] 2.2 Add build_sql() match arm for Stats
- [x] 2.3 Add build_sql() match arm for GroupBy

### Phase 3: Tests
- [ ] 3.1 Add parser test: test_parse_aggregate
- [ ] 3.2 Add parser test: test_parse_aggregate_avg
- [ ] 3.3 Add parser test: test_parse_stats_stddev
- [ ] 3.4 Add parser test: test_parse_stats_median
- [ ] 3.5 Add parser test: test_parse_stats_percentile
- [ ] 3.6 Add parser test: test_parse_stats_percentile_0
- [ ] 3.7 Add parser test: test_parse_group_by
- [ ] 3.8 Add parser test: test_parse_aggregate_missing_property (negative)
- [ ] 3.9 Add parser test: test_parse_aggregate_missing_fn (negative)
- [ ] 3.10 Add parser test: test_parse_stats_percentile_oob (negative)
- [ ] 3.11 Add executor test: test_aggregate_count_sql
- [ ] 3.12 Add executor test: test_stats_stddev_sql
- [ ] 3.13 Add executor test: test_stats_median_sql
- [ ] 3.14 Add executor test: test_stats_percentile_sql
- [ ] 3.15 Add executor test: test_group_by_sql

### Phase 4: Integration
- [x] 4.1 Update lib.rs exports (AggregateFn, StatsFn, QueryError)
- [x] 4.2 Run cargo test -p quilt-query
- [x] 4.3 Run cargo clippy -p quilt-query
- [x] 4.4 Run cargo fmt --check
