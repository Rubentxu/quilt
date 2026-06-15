//! MCP handler for semantic property relations (PI-8).

use async_trait::async_trait;
use quilt_domain::properties::relation::{PropertyRelation, RelationType};
use quilt_domain::repositories::RelationRepository;
use quilt_domain::value_objects::Uuid;
use serde_json::Value;
use std::sync::Arc;

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;

/// MCP tool handler for property relations.
pub struct RelationHandler {
    repo: Arc<dyn RelationRepository>,
}

impl RelationHandler {
    pub fn new(repo: Arc<dyn RelationRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ToolHandler for RelationHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "quilt_properties_relations".to_string(),
            description: "Manage semantic property relations: workflows, hierarchies, implications between property values".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "get_by_key", "get_from", "create", "delete"],
                        "description": "Action to perform"
                    },
                    "id": { "type": "string", "description": "Relation UUID (for get, delete)" },
                    "key": { "type": "string", "description": "Property key (for get_by_key)" },
                    "value": { "type": "string", "description": "Property value (for get_from)" },
                    "source_key": { "type": "string", "description": "Source property key (for create)" },
                    "source_value": { "type": "string", "description": "Source property value (for create)" },
                    "target_key": { "type": "string", "description": "Target property key (for create)" },
                    "target_value": { "type": "string", "description": "Target property value (for create)" },
                    "relation_type": { "type": "string", "description": "Type: precedes|broadens|implies|requires|custom", "default": "precedes" },
                    "description": { "type": "string", "description": "Relation description" },
                    "confidence": { "type": "number", "description": "Confidence 0-1", "default": 1.0 }
                },
                "required": ["action"]
            }),
        }]
    }

    #[tracing::instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_properties_relations" => {
                let action = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'action'")?;

                match action {
                    "list" => {
                        let rels = self.repo.list_all().await.map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&serde_json::json!({
                            "count": rels.len(),
                            "relations": rels,
                        }))
                        .unwrap_or_else(|e| format!("Error: {}", e)))
                    }
                    "get_by_key" => {
                        let key = args
                            .get("key")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'key'")?;
                        let rels = self.repo.get_by_key(key).await.map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&serde_json::json!({
                            "count": rels.len(),
                            "relations": rels,
                        }))
                        .unwrap_or_else(|e| format!("Error: {}", e)))
                    }
                    "get_from" => {
                        let key = args
                            .get("key")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'key'")?;
                        let value = args
                            .get("value")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'value'")?;
                        let rels = self
                            .repo
                            .get_from(key, value)
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&serde_json::json!({
                            "count": rels.len(),
                            "relations": rels,
                        }))
                        .unwrap_or_else(|e| format!("Error: {}", e)))
                    }
                    "create" => {
                        let source_key = args
                            .get("source_key")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'source_key'")?;
                        let source_value = args
                            .get("source_value")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'source_value'")?;
                        let target_key = args
                            .get("target_key")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'target_key'")?;
                        let target_value = args
                            .get("target_value")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'target_value'")?;
                        let rt_str = args
                            .get("relation_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("precedes");
                        let desc = args
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let conf = args
                            .get("confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0);

                        let rt = match rt_str {
                            "precedes" => RelationType::Precedes,
                            "broadens" => RelationType::Broadens,
                            "implies" => RelationType::Implies,
                            "requires" => RelationType::Requires,
                            other => RelationType::Custom(other.to_string()),
                        };

                        let relation = PropertyRelation::new(
                            Uuid::new_v4(),
                            source_key.to_string(),
                            source_value.to_string(),
                            target_key.to_string(),
                            target_value.to_string(),
                            rt,
                            desc.to_string(),
                            conf,
                        );
                        self.repo
                            .insert(&relation)
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok(serde_json::to_string_pretty(&relation)
                            .unwrap_or_else(|e| format!("Error: {}", e)))
                    }
                    "delete" => {
                        let id_str = args
                            .get("id")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'id'")?;
                        let id = id_str.parse::<Uuid>().map_err(|e| e.to_string())?;
                        self.repo.delete(id).await.map_err(|e| e.to_string())?;
                        Ok(
                            serde_json::to_string_pretty(&serde_json::json!({"deleted": true}))
                                .unwrap(),
                        )
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
