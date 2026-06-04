//! Graph tool handler (G4 — Static graph construction).
//!
//! Owns: `quilt_graph_edges`.
//!
//! Returns typed edges for a given block at depth=1. Depth > 1 returns
//! a concrete "V2" error indicating the recursive walker is not yet
//! implemented.

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use crate::use_cases::BlockUseCases;
use async_trait::async_trait;
use quilt_domain::references::{EdgeType, TypedEdge};
use quilt_domain::value_objects::Uuid;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Graph tool handler — typed edge retrieval for G4.
pub struct GraphToolHandler {
    block_use_cases: Arc<dyn BlockUseCases>,
}

impl GraphToolHandler {
    pub fn new(block_use_cases: Arc<dyn BlockUseCases>) -> Self {
        Self { block_use_cases }
    }
}

#[async_trait]
impl ToolHandler for GraphToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "quilt_graph_edges".to_string(),
            description: concat!(
                "Get typed edges for a block. Returns all outgoing edges from a block ",
                "as TypedEdge structs (from, to, edge_type, weight). ",
                "Depth > 1 returns V2 error (recursive walker not yet implemented)."
            )
            .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "block_id": {
                        "type": "string",
                        "description": "UUID of the source block."
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Traversal depth. depth=1 returns immediate edges. depth>1 returns V2 error.",
                        "default": 1
                    }
                },
                "required": ["block_id"]
            }),
        }]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_graph_edges" => {
                let block_id_str = args
                    .get("block_id")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'block_id' parameter")?;
                let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

                // G4 V1: depth > 1 → concrete "V2" error (not TODO)
                if depth > 1 {
                    return Err("V2".to_string());
                }

                // Parse block UUID
                let block_id = Uuid::parse_str(block_id_str)
                    .ok_or_else(|| "Invalid block_id: must be a UUID")?;

                // Get block tree at depth=1 (immediate children)
                let tree = self
                    .block_use_cases
                    .get_tree(block_id)
                    .await
                    .map_err(|e| e.to_string())?;

                // Map tree children to TypedEdges.
                // Each child is a BlockRef edge from parent → child.
                // Since we don't have the full RefIndex here, we use BlockRef as the
                // default edge type. Full RefType resolution requires the RefService.
                let edges: Vec<serde_json::Value> = tree
                    .children
                    .iter()
                    .map(|child| {
                        let edge = TypedEdge::new(block_id, child.id, EdgeType::BlockRef, 1.0, 0);
                        serde_json::json!({
                            "from": edge.from.to_string(),
                            "to": edge.to.to_string(),
                            "edge_type": format!("{:?}", edge.edge_type),
                            "weight": edge.weight,
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "count": edges.len(),
                    "edges": edges,
                    "note": "V1: edge types are BlockRef. Full RefType resolution requires RefService.",
                }))
                .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // G4: evidence includes block IDs from the edge list.
    fn tool_evidence(&self, name: &str, _args: &Value, result: &Value) -> Option<Evidence> {
        let mut ev = Evidence::universal_fallback(name);
        match name {
            "quilt_graph_edges" => {
                if let Some(edges) = result.get("edges").and_then(|v| v.as_array()) {
                    for edge in edges {
                        if let Some(from) = edge.get("from").and_then(|v| v.as_str()) {
                            if let Some(uuid) = Uuid::parse_str(from) {
                                ev.block_ids.push(uuid.into()); // domain Uuid → uuid::Uuid
                            }
                        }
                        if let Some(to) = edge.get("to").and_then(|v| v.as_str()) {
                            if let Some(uuid) = Uuid::parse_str(to) {
                                ev.block_ids.push(uuid.into()); // domain Uuid → uuid::Uuid
                            }
                        }
                    }
                }
                Some(ev)
            }
            _ => None,
        }
    }
}
