# Tasks: DSL `analyze` Operator (Change B2)

## Task List

### Phase 1: Parser
- [x] 1.1 Add AnalyzeKind enum to parser.rs
- [x] 1.2 Add QueryExpr::Analyze variant
- [x] 1.3 Wire "analyze" in parse_compound() match
- [x] 1.4 Add parse_analyze() method
- [x] 1.5 Add parse_analyze_kind() method

### Phase 2: Executor
- [x] 2.1 Add AnalyzeResult enum to executor.rs
- [x] 2.2 Add AnalyzeError enum to executor.rs
- [x] 2.3 Add build_analyze_sql() method to QueryExecutor

### Phase 3: QueryService
- [x] 3.1 Add prepare_analyze() to QueryService
- [x] 3.2 Add execute_analyze() async method to QueryService

### Phase 4: MCP Integration
- [x] 4.1 Update tool_quilt_query() routing in server.rs

### Phase 5: Tests
- [x] 5.1 Add parser test: test_parse_analyze_cognitive_mirror
- [x] 5.2 Add parser test: test_parse_analyze_serendipity_defaults
- [x] 5.3 Add parser test: test_parse_analyze_serendipity_with_limit
- [x] 5.4 Add parser test: test_parse_analyze_serendipity_full
- [x] 5.5 Add parser test: test_parse_analyze_empty (negative)
- [x] 5.6 Add parser test: test_parse_analyze_missing_kind (negative)
- [x] 5.7 Add parser test: test_parse_analyze_unknown_kind (negative)
- [x] 5.8 Add parser test: test_parse_analyze_limit_no_value (negative)
- [x] 5.9 Add parser test: test_parse_analyze_min_confidence_no_value (negative)
- [x] 5.10 Add parser test: test_parse_analyze_temporal_window_no_value (negative)
- [x] 5.11 Add parser test: test_parse_analyze_cognitive_mirror_with_kwargs (negative)
- [x] 5.12 Add executor test: test_build_analyze_sql_simple
- [x] 5.13 Add executor test: test_build_analyze_sql_page_filter
- [x] 5.14 Add executor test: test_build_analyze_sql_non_analyze_error
- [x] 5.15 Add query_service test: test_prepare_analyze_round_trip
- [x] 5.16 Add query_service test: test_prepare_analyze_non_analyze_error

### Phase 6: Integration
- [x] 6.1 Update lib.rs exports (AnalyzeKind, AnalyzeResult, AnalyzeError)
- [x] 6.2 Add new dependencies to quilt-query Cargo.toml if needed
- [x] 6.3 Run cargo check -p quilt-query -p quilt-application
- [x] 6.4 Run cargo test -p quilt-query -p quilt-application
- [x] 6.5 Run cargo clippy -p quilt-query -p quilt-application
- [x] 6.6 Run cargo fmt --check
