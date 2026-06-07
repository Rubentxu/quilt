//! System tool handler
//!
//! Owns: quilt_list_property_types, quilt_get_query_capabilities

use crate::handlers::ToolHandler;
use crate::tools::Tool;
use async_trait::async_trait;
use quilt_domain::properties::builtin::get_all_builtin_properties;
use quilt_domain::properties::types::{Cardinality, PropertyType};
use serde_json::Value;
use tracing::instrument;

/// System tool handler.
///
/// Provides tools for querying system-wide metadata:
/// - `quilt_list_property_types`: List all available property types
/// - `quilt_get_query_capabilities`: Query DSL and search capabilities
pub struct SystemToolHandler {
    _priv: (),
}

impl SystemToolHandler {
    /// Create a new system tool handler.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl Default for SystemToolHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for SystemToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_list_property_types".to_string(),
                description: "List all available property types in the system".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "quilt_get_query_capabilities".to_string(),
                description: "Get the query and search capabilities supported by the system"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    #[instrument(skip(self, _args))]
    async fn execute(&self, name: &str, _args: &Value) -> Result<String, String> {
        match name {
            "quilt_list_property_types" => self.list_property_types(),
            "quilt_get_query_capabilities" => self.get_query_capabilities(),
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}

impl SystemToolHandler {
    /// List all available property types (builtin + custom).
    fn list_property_types(&self) -> Result<String, String> {
        let builtin_props = get_all_builtin_properties();

        let property_types: Vec<serde_json::Value> = builtin_props
            .iter()
            .map(|def| {
                let type_str = match def.property_type {
                    PropertyType::Text => "Text",
                    PropertyType::Number => "Number",
                    PropertyType::Date => "Date",
                    PropertyType::DateTime => "DateTime",
                    PropertyType::Url => "Url",
                    PropertyType::Checkbox => "Checkbox",
                    PropertyType::Node => "Node",
                };

                let cardinality_str = match def.cardinality {
                    Cardinality::One => "One",
                    Cardinality::Many => "Many",
                };

                let closed_values: Option<Vec<serde_json::Value>> = if def.closed_values.is_empty()
                {
                    None
                } else {
                    Some(
                        def.closed_values
                            .iter()
                            .map(|cv| {
                                serde_json::json!({
                                    "id": cv.id.to_string(),
                                    "db_ident": cv.db_ident,
                                    "value": cv.value,
                                    "icon": cv.icon,
                                    "order": cv.order,
                                })
                            })
                            .collect(),
                    )
                };

                serde_json::json!({
                    "id": def.id.to_string(),
                    "db_ident": def.db_ident,
                    "title": def.title,
                    "property_type": type_str,
                    "cardinality": cardinality_str,
                    "closed_values": closed_values,
                    "view_context": def.view_context.as_str(),
                    "public": def.public,
                    "queryable": def.queryable,
                    "hidden": def.hidden,
                    "read_only": def.read_only,
                    "source": "builtin",
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "count": property_types.len(),
            "property_types": property_types,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    /// Get query and search capabilities supported by the system.
    fn get_query_capabilities(&self) -> Result<String, String> {
        // Query operators from PropertyOp enum
        let property_operators = vec![
            serde_json::json!({
                "name": "equals",
                "symbol": "=",
                "description": "Property equals a value",
                "arity": 2
            }),
            serde_json::json!({
                "name": "not_equals",
                "symbol": "!=",
                "description": "Property does not equal a value",
                "arity": 2
            }),
            serde_json::json!({
                "name": "contains",
                "symbol": "contains",
                "description": "Property contains a substring",
                "arity": 2
            }),
            serde_json::json!({
                "name": "greater_than",
                "symbol": ">",
                "description": "Property greater than a value",
                "arity": 2
            }),
            serde_json::json!({
                "name": "less_than",
                "symbol": "<",
                "description": "Property less than a value",
                "arity": 2
            }),
            serde_json::json!({
                "name": "greater_than_or_equal",
                "symbol": ">=",
                "description": "Property greater than or equal to a value",
                "arity": 2
            }),
            serde_json::json!({
                "name": "less_than_or_equal",
                "symbol": "<=",
                "description": "Property less than or equal to a value",
                "arity": 2
            }),
            serde_json::json!({
                "name": "between",
                "symbol": "between",
                "description": "Property between two values",
                "arity": 3
            }),
        ];

        // QueryAst variants (DSL operations)
        let dsl_operations = vec![
            serde_json::json!({
                "name": "And",
                "syntax": "and(expr1, expr2, ...)",
                "description": "Boolean AND of multiple expressions"
            }),
            serde_json::json!({
                "name": "Or",
                "syntax": "or(expr1, expr2, ...)",
                "description": "Boolean OR of multiple expressions"
            }),
            serde_json::json!({
                "name": "Not",
                "syntax": "not(expr)",
                "description": "Boolean NOT of an expression"
            }),
            serde_json::json!({
                "name": "Between",
                "syntax": "between(field, start, end)",
                "description": "Range filter between two values"
            }),
            serde_json::json!({
                "name": "Property",
                "syntax": "property(key, op, value)",
                "description": "JSON property filter with operator"
            }),
            serde_json::json!({
                "name": "Task",
                "syntax": "task(todo|doing|done|...)",
                "description": "Task marker filter"
            }),
            serde_json::json!({
                "name": "Priority",
                "syntax": "priority(a|b|c)",
                "description": "Priority filter"
            }),
            serde_json::json!({
                "name": "Page",
                "syntax": "page(Page Name)",
                "description": "Page name filter"
            }),
            serde_json::json!({
                "name": "Tags",
                "syntax": "tags(tag)",
                "description": "Tag filter"
            }),
            serde_json::json!({
                "name": "PageRef",
                "syntax": "[[Page Name]]",
                "description": "Page reference filter"
            }),
            serde_json::json!({
                "name": "BlockContent",
                "syntax": "content(text)",
                "description": "Full-text search content"
            }),
            serde_json::json!({
                "name": "Exists",
                "syntax": "exists(property)",
                "description": "Filter to items where property exists"
            }),
            serde_json::json!({
                "name": "Missing",
                "syntax": "missing(property)",
                "description": "Filter to items where property is missing"
            }),
            serde_json::json!({
                "name": "SortBy",
                "syntax": "sort(field, asc|desc, expr)",
                "description": "Sort results by field and direction"
            }),
            serde_json::json!({
                "name": "Sample",
                "syntax": "sample(n)",
                "description": "Random sample of n results"
            }),
            serde_json::json!({
                "name": "Aggregate",
                "syntax": "aggregate(inner, group_by, fn)",
                "description": "Aggregate with GROUP BY"
            }),
            serde_json::json!({
                "name": "Stats",
                "syntax": "stats(property, fn)",
                "description": "Statistical computation over a property"
            }),
            serde_json::json!({
                "name": "PageFuzzy",
                "syntax": "pagefuzzy(term, limit)",
                "description": "Fuzzy page name search"
            }),
            serde_json::json!({
                "name": "Temporal",
                "syntax": "temporal(range, inner)",
                "description": "Filter by temporal range"
            }),
            serde_json::json!({
                "name": "VirtualSelect",
                "syntax": "select(columns, inner)",
                "description": "Select with virtual columns"
            }),
        ];

        // Sort modes
        let sort_modes = vec![
            serde_json::json!({
                "direction": "asc",
                "description": "Ascending order"
            }),
            serde_json::json!({
                "direction": "desc",
                "description": "Descending order"
            }),
        ];

        // Search features
        let search_features = serde_json::json!({
            "full_text_search": {
                "enabled": true,
                "engine": "FTS5",
                "features": [
                    "bm25_ranking",
                    "prefix_matching",
                    "phrase_search",
                    "boolean_operators",
                    "case_insensitive"
                ]
            },
            "fuzzy_search": {
                "enabled": true,
                "description": "Prefix matching with LIKE fallback"
            },
            "query_language": {
                "enabled": true,
                "name": "Quilt DSL",
                "syntax": "S-expression based"
            }
        });

        // Aggregate functions
        let aggregate_functions = vec![
            serde_json::json!({"name": "count", "description": "Count of items"}),
            serde_json::json!({"name": "avg", "description": "Average of numeric values"}),
            serde_json::json!({"name": "sum", "description": "Sum of numeric values"}),
            serde_json::json!({"name": "min", "description": "Minimum value"}),
            serde_json::json!({"name": "max", "description": "Maximum value"}),
        ];

        // Statistical functions
        let stats_functions = vec![
            serde_json::json!({"name": "stddev", "description": "Standard deviation"}),
            serde_json::json!({"name": "variance", "description": "Variance"}),
            serde_json::json!({"name": "median", "description": "Median value"}),
            serde_json::json!({"name": "percentile", "description": "Arbitrary percentile (0-100)"}),
        ];

        // Temporal ranges
        let temporal_ranges = vec![
            serde_json::json!({"name": "Today", "description": "Current calendar day"}),
            serde_json::json!({"name": "Yesterday", "description": "One day before today"}),
            serde_json::json!({"name": "ThisWeek", "description": "This week (Monday to today)"}),
            serde_json::json!({"name": "LastWeek", "description": "Previous week (Monday to Sunday)"}),
            serde_json::json!({"name": "ThisMonth", "description": "This month (day 1 to today)"}),
            serde_json::json!({"name": "LastMonth", "description": "Previous month"}),
            serde_json::json!({"name": "Custom", "description": "Custom date range"}),
            serde_json::json!({"name": "Relative", "description": "Relative offset from now"}),
        ];

        // Virtual columns (F12)
        let virtual_columns = vec![
            serde_json::json!({"name": "word_count", "description": "Word count of content"}),
            serde_json::json!({"name": "char_count", "description": "Character count"}),
            serde_json::json!({"name": "ref_count", "description": "Reference count"}),
            serde_json::json!({"name": "block_age_days", "description": "Block age in days"}),
        ];

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "query": {
                "operators": property_operators,
                "operations": dsl_operations,
                "sort_modes": sort_modes,
                "aggregate_functions": aggregate_functions,
                "stats_functions": stats_functions,
                "temporal_ranges": temporal_ranges,
                "virtual_columns": virtual_columns,
            },
            "search": search_features,
            "limits": {
                "max_query_limit": 10000,
                "default_query_limit": 100,
                "max_search_limit": 1000,
                "default_search_limit": 50,
                "max_fuzzy_limit": 100,
            }
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_handler() -> SystemToolHandler {
        SystemToolHandler::new()
    }

    #[test]
    fn test_tools_registered() {
        let handler = create_handler();
        let tools = handler.tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().any(|t| t.name == "quilt_list_property_types"));
        assert!(
            tools
                .iter()
                .any(|t| t.name == "quilt_get_query_capabilities")
        );
    }

    #[tokio::test]
    async fn test_list_property_types() {
        let handler = create_handler();
        let result = handler
            .execute("quilt_list_property_types", &serde_json::json!({}))
            .await;
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(json.get("count").is_some());
        assert!(json.get("property_types").is_some());
        let property_types = json["property_types"].as_array().unwrap();
        // Should have builtin properties
        assert!(!property_types.is_empty());
        // Verify structure of a property type
        let first = &property_types[0];
        assert!(first.get("id").is_some());
        assert!(first.get("db_ident").is_some());
        assert!(first.get("title").is_some());
        assert!(first.get("property_type").is_some());
        assert!(first.get("cardinality").is_some());
        assert!(first.get("source").is_some());
        assert_eq!(first["source"], "builtin");
    }

    #[tokio::test]
    async fn test_list_property_types_builtin_props() {
        let handler = create_handler();
        let result = handler
            .execute("quilt_list_property_types", &serde_json::json!({}))
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        let property_types = json["property_types"].as_array().unwrap();

        // Check that status property exists and has closed values
        let status = property_types
            .iter()
            .find(|p| p["db_ident"] == "quilt.property/status");
        assert!(status.is_some());
        let status = status.unwrap();
        assert!(!status["closed_values"].is_null());
        let closed_values = status["closed_values"].as_array().unwrap();
        assert!(!closed_values.is_empty());
    }

    #[tokio::test]
    async fn test_get_query_capabilities() {
        let handler = create_handler();
        let result = handler
            .execute("quilt_get_query_capabilities", &serde_json::json!({}))
            .await;
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();

        // Query section
        assert!(json.get("query").is_some());
        let query = &json["query"];
        assert!(query["operators"].is_array());
        assert!(query["operations"].is_array());
        assert!(query["sort_modes"].is_array());
        assert!(query["aggregate_functions"].is_array());
        assert!(query["stats_functions"].is_array());
        assert!(query["temporal_ranges"].is_array());

        // Search section
        assert!(json.get("search").is_some());
        let search = &json["search"];
        assert!(search["full_text_search"].is_object());
        assert!(search["fuzzy_search"].is_object());
        assert!(search["query_language"].is_object());

        // Limits section
        assert!(json.get("limits").is_some());
        let limits = &json["limits"];
        assert!(limits["max_query_limit"].is_number());
        assert!(limits["default_query_limit"].is_number());
        assert!(limits["max_search_limit"].is_number());
        assert!(limits["default_search_limit"].is_number());
    }

    #[tokio::test]
    async fn test_get_query_capabilities_operators() {
        let handler = create_handler();
        let result = handler
            .execute("quilt_get_query_capabilities", &serde_json::json!({}))
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        let operators = json["query"]["operators"].as_array().unwrap();

        // Should have all PropertyOp variants
        assert!(operators.len() >= 8);
        let names: Vec<&str> = operators
            .iter()
            .filter_map(|op| op["name"].as_str())
            .collect();
        assert!(names.contains(&"equals"));
        assert!(names.contains(&"not_equals"));
        assert!(names.contains(&"contains"));
        assert!(names.contains(&"greater_than"));
        assert!(names.contains(&"less_than"));
        assert!(names.contains(&"between"));
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let handler = create_handler();
        let result = handler
            .execute("quilt_unknown_tool", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }
}
