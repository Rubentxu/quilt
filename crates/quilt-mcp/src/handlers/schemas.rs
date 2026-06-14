//! MCP handler for property schema operations (PI-7).

use async_trait::async_trait;
use quilt_application::schema::{SchemaService, SchemaServiceTrait};
use quilt_domain::properties::schema::{AutoDetectParams, PropertySchema};
use quilt_domain::value_objects::Uuid;
use serde_json::Value;


use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;

/// MCP tool handler for property schema operations.
pub struct SchemaHandler {
    service: SchemaService,
}

impl SchemaHandler {
    pub fn new(service: SchemaService) -> Self {
        Self { service }
    }
}

#[async_trait]
impl ToolHandler for SchemaHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "quilt_properties_schemas".to_string(),
            description: "Manage property schemas: list, create, get, delete, or auto-detect property clusters".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "get", "get_by_name", "create", "delete", "auto_detect"],
                        "description": "Action to perform"
                    },
                    "name": {
                        "type": "string",
                        "description": "Schema name (for create, get_by_name)"
                    },
                    "id": {
                        "type": "string",
                        "description": "Schema UUID (for get, delete)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description (for create)"
                    },
                    "property_keys": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Property keys (for create)"
                    },
                    "min_co_occurrence": {
                        "type": "integer",
                        "description": "Min co-occurrence for auto-detect",
                        "default": 3
                    },
                    "min_pmi": {
                        "type": "number",
                        "description": "Min PMI score for auto-detect",
                        "default": 0.5
                    },
                    "max_schemas": {
                        "type": "integer",
                        "description": "Max schemas to detect",
                        "default": 10
                    }
                },
                "required": ["action"]
            }),
        }]
    }

    #[tracing::instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_properties_schemas" => {
                let action = args.get("action").and_then(|v| v.as_str()).ok_or("Missing 'action'")?;

                match action {
                    "list" => {
                        let schemas = self.service.list_all().await.map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&serde_json::json!({
                            "count": schemas.len(),
                            "schemas": schemas,
                        })).unwrap_or_else(|e| format!("Error: {}", e)))
                    }
                    "get" => {
                        let id_str = args.get("id").and_then(|v| v.as_str()).ok_or("Missing 'id'")?;
                        let id = id_str.parse::<Uuid>().map_err(|e| e.to_string())?;
                        let schema = self.service.get_by_id(id).await.map_err(|e| e.to_string())?;
                        match schema {
                            Some(s) => Ok(serde_json::to_string_pretty(&s).unwrap_or_else(|e| format!("Error: {}", e))),
                            None => Ok("Schema not found".to_string()),
                        }
                    }
                    "get_by_name" => {
                        let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing 'name'")?;
                        let schema = self.service.get_by_name(name).await.map_err(|e| e.to_string())?;
                        match schema {
                            Some(s) => Ok(serde_json::to_string_pretty(&s).unwrap_or_else(|e| format!("Error: {}", e))),
                            None => Ok("Schema not found".to_string()),
                        }
                    }
                    "create" => {
                        let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing 'name'")?;
                        let description = args.get("description").and_then(|v| v.as_str()).unwrap_or("");
                        let keys: Vec<String> = args.get("property_keys")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default();

                        let schema = PropertySchema::new(
                            Uuid::new_v4(),
                            name.to_string(),
                            description.to_string(),
                            keys,
                            false,
                        );
                        self.service.create(&schema).await.map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&schema).unwrap_or_else(|e| format!("Error: {}", e)))
                    }
                    "delete" => {
                        let id_str = args.get("id").and_then(|v| v.as_str()).ok_or("Missing 'id'")?;
                        let id = id_str.parse::<Uuid>().map_err(|e| e.to_string())?;
                        self.service.delete(id).await.map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&serde_json::json!({"deleted": true})).unwrap())
                    }
                    "auto_detect" => {
                        let params = AutoDetectParams {
                            min_co_occurrence: args.get("min_co_occurrence").and_then(|v| v.as_u64()).unwrap_or(3),
                            min_pmi: args.get("min_pmi").and_then(|v| v.as_f64()).unwrap_or(0.5),
                            max_schemas: args.get("max_schemas").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
                            min_properties: 2,
                        };
                        let detected = self.service.auto_detect(&params).await.map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&serde_json::json!({
                            "count": detected.len(),
                            "detected": detected,
                        })).unwrap_or_else(|e| format!("Error: {}", e)))
                    }
                    _ => Err(format!("Unknown action: {}", action)),
                }
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    fn tool_evidence(&self, _name: &str, _args: &Value, _result: &Value) -> Option<Evidence> {
        None
    }
}
