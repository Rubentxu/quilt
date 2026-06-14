//! Cognitive Analysis handler — wires quilt-analysis engines to MCP tools.
//!
//! Implements the 5 cognitive analysis tools:
//! - `quilt_analyze_connections`   → ConnectionEngine
//! - `quilt_analyze_clusters`      → StructuralMirror (clusters)
//! - `quilt_analyze_centrality`    → StructuralMirror (PageRank influence)
//! - `quilt_analyze_structure`     → StructureMapper
//! - `quilt_garden_health`        → StructureGardener + TemplateDoctor

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use crate::use_cases::BlockUseCases;
use async_trait::async_trait;
use quilt_analysis::connection_engine::{ConnectionEngine, SerendipityQuery};
use quilt_analysis::structure_gardener::StructureGardener;
use quilt_analysis::structure_mapper::StructureMapper;
use quilt_analysis::structural_mirror::StructuralMirror;
use quilt_domain::value_objects::Uuid;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Cognitive analysis tool handler — wires quilt-analysis engines to MCP.
#[derive(Clone)]
pub struct CognitiveToolHandler {
    block_use_cases: Arc<dyn BlockUseCases>,
    connection_engine: ConnectionEngine,
    structural_mirror: StructuralMirror,
    structure_mapper: StructureMapper,
    structure_gardener: StructureGardener,
}

impl CognitiveToolHandler {
    pub fn new(
        block_use_cases: Arc<dyn BlockUseCases>,
        connection_engine: ConnectionEngine,
        structural_mirror: StructuralMirror,
        structure_mapper: StructureMapper,
        structure_gardener: StructureGardener,
    ) -> Self {
        Self {
            block_use_cases,
            connection_engine,
            structural_mirror,
            structure_mapper,
            structure_gardener,
        }
    }
}

#[async_trait]
impl ToolHandler for CognitiveToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_analyze_connections".to_string(),
                description: concat!(
                    "Find unexpected connections between blocks within a page. ",
                    "Uses structural similarity (Jaccard index on shared refs) and ",
                    "temporal proximity (exponential decay by age) to discover serendipitous links. ",
                    "Returns connections sorted by confidence score."
                )
                .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_id": {
                            "type": "string",
                            "description": "UUID of the page to analyze for connections."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of connections to return (default 20).",
                            "default": 20
                        },
                        "min_confidence": {
                            "type": "number",
                            "description": "Minimum confidence threshold 0.0–1.0 (default 0.3).",
                            "default": 0.3
                        },
                        "temporal_window_days": {
                            "type": "integer",
                            "description": "Optional: analyze blocks updated within N days instead of by page."
                        }
                    },
                    "required": ["page_id"]
                }),
            },
            Tool {
                name: "quilt_analyze_clusters".to_string(),
                description: concat!(
                    "Detect knowledge clusters in a page's block reference graph. ",
                    "Uses connected components to group densely-connected blocks. ",
                    "Returns clusters with coherence scores, density per block, ",
                    "frontier blocks (many outgoing refs), and structural gaps."
                )
                .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_id": {
                            "type": "string",
                            "description": "UUID of the page to analyze for clusters."
                        }
                    },
                    "required": ["page_id"]
                }),
            },
            Tool {
                name: "quilt_analyze_centrality".to_string(),
                description: concat!(
                    "Compute PageRank-style centrality scores for all blocks in the graph. ",
                    "Returns influence scores sorted descending. ",
                    "Optionally filter to a specific block's score."
                )
                .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": {
                            "type": "string",
                            "description": "Optional: UUID of a specific block to get its centrality score."
                        },
                        "top_n": {
                            "type": "integer",
                            "description": "Maximum number of blocks to return sorted by score (default 20).",
                            "default": 20
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "quilt_analyze_structure".to_string(),
                description: concat!(
                    "Analyze the argument structure of a page's blocks. ",
                    "Classifies blocks as Claim/Evidence/Rebuttal/Qualification/Assumption, ",
                    "builds typed edges (supports/refutes/qualifies), detects consensus zones, ",
                    "and scores individual argument strength. Optionally detect logical fallacies."
                )
                .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_id": {
                            "type": "string",
                            "description": "UUID of the page to analyze for structure."
                        },
                        "detect_fallacies": {
                            "type": "boolean",
                            "description": "Also detect logical fallacies in blocks (default false).",
                            "default": false
                        },
                        "block_id": {
                            "type": "string",
                            "description": "Optional: analyze a single block's argument strength instead of whole page."
                        }
                    },
                    "required": ["page_id"]
                }),
            },
            Tool {
                name: "quilt_garden_health".to_string(),
                description: concat!(
                    "Check structure health by tracking belief evolution, detecting contradictions, ",
                    "and suggesting areas for deeper exploration. Works across all journal pages. ",
                    "Returns contradictions, deepening suggestions, and mental model summary."
                )
                .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "agent_id": {
                            "type": "string",
                            "description": "Agent ID to check health for (default 'default').",
                            "default": "default"
                        },
                        "depth_threshold": {
                            "type": "integer",
                            "description": "Minimum belief depth to be considered healthy (default 3).",
                            "default": 3
                        }
                    },
                    "required": []
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_analyze_connections" => self.execute_connections(args).await,
            "quilt_analyze_clusters" => self.execute_clusters(args).await,
            "quilt_analyze_centrality" => self.execute_centrality(args).await,
            "quilt_analyze_structure" => self.execute_structure(args).await,
            "quilt_garden_health" => self.execute_garden_health(args).await,
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    fn tool_evidence(&self, name: &str, _args: &Value, result: &Value) -> Option<Evidence> {
        let mut ev = Evidence::universal_fallback(name);
        match name {
            "quilt_analyze_connections" => {
                if let Some(conns) = result.get("connections").and_then(|v| v.as_array()) {
                    for conn in conns {
                        if let Some(a) = conn.get("idea_a").and_then(|v| v.as_str()).and_then(Uuid::parse_str) {
                            ev.block_ids.push(a.into());
                        }
                        if let Some(b) = conn.get("idea_b").and_then(|v| v.as_str()).and_then(Uuid::parse_str) {
                            ev.block_ids.push(b.into());
                        }
                    }
                }
                Some(ev)
            }
            "quilt_analyze_clusters" => {
                if let Some(clusters) = result.get("clusters").and_then(|v| v.as_array()) {
                    for cluster in clusters {
                        if let Some(ids) = cluster.get("block_ids").and_then(|v| v.as_array()) {
                            for id in ids {
                                if let Some(s) = id.as_str().and_then(Uuid::parse_str) {
                                    ev.block_ids.push(s.into());
                                }
                            }
                        }
                    }
                }
                Some(ev)
            }
            "quilt_analyze_centrality" => {
                if let Some(scores) = result.get("scores").and_then(|v| v.as_array()) {
                    for score in scores {
                        if let Some(s) = score.get("block_id").and_then(|v| v.as_str()).and_then(Uuid::parse_str) {
                            ev.block_ids.push(s.into());
                        }
                    }
                }
                Some(ev)
            }
            "quilt_analyze_structure" => {
                if let Some(nodes) = result.get("nodes").and_then(|v| v.as_array()) {
                    for node in nodes {
                        if let Some(s) = node.get("block_id").and_then(|v| v.as_str()).and_then(Uuid::parse_str) {
                            ev.block_ids.push(s.into());
                        }
                    }
                }
                Some(ev)
            }
            _ => None,
        }
    }
}

impl CognitiveToolHandler {
    async fn execute_connections(&self, args: &Value) -> Result<String, String> {
        let page_id_str = args
            .get("page_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'page_id' parameter")?;
        let page_id =
            Uuid::parse_str(page_id_str).ok_or("Invalid page_id: must be a UUID")?;

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;
        let min_confidence = args
            .get("min_confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3) as f32;
        let temporal_window_days = args
            .get("temporal_window_days")
            .and_then(|v| v.as_i64())
            .map(|d| d as i64);

        let query = SerendipityQuery {
            topic: None,
            limit,
            offset: 0,
            min_confidence,
            temporal_window_days,
            page_id: if temporal_window_days.is_some() {
                None
            } else {
                Some(page_id)
            },
        };

        let connections = self
            .connection_engine
            .find_connections(query)
            .await
            .map_err(|e| e.to_string())?;

        let connections_json: Vec<serde_json::Value> = connections
            .iter()
            .map(|c| {
                serde_json::json!({
                    "idea_a": c.idea_a.to_string(),
                    "idea_b": c.idea_b.to_string(),
                    "bridge_concept": c.bridge_concept,
                    "confidence": c.confidence,
                    "explanation": c.explanation,
                    "connection_type": format!("{:?}", c.connection_type).to_lowercase(),
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "count": connections_json.len(),
            "connections": connections_json,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn execute_clusters(&self, args: &Value) -> Result<String, String> {
        let page_id_str = args
            .get("page_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'page_id' parameter")?;
        let page_id =
            Uuid::parse_str(page_id_str).ok_or("Invalid page_id: must be a UUID")?;

        let structure_map = self
            .structural_mirror
            .analyze(page_id)
            .await
            .map_err(|e| e.to_string())?;

        let clusters_json: Vec<serde_json::Value> = structure_map
            .clusters
            .iter()
            .map(|c| {
                serde_json::json!({
                    "block_ids": c.block_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
                    "theme": c.theme,
                    "coherence_score": c.coherence_score,
                })
            })
            .collect();

        let density_json: std::collections::HashMap<String, f32> = structure_map
            .density
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();

        let gaps_json: Vec<serde_json::Value> = structure_map
            .gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "from": g.from.to_string(),
                    "to": g.to.to_string(),
                    "shared_refs": g.shared_refs.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "cluster_count": clusters_json.len(),
            "clusters": clusters_json,
            "density": density_json,
            "frontiers": structure_map.frontiers.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
            "gaps": gaps_json,
            "influences": structure_map.influences.iter().map(|i| {
                serde_json::json!({
                    "block_id": i.block_id.to_string(),
                    "influence_score": i.influence_score,
                })
            }).collect::<Vec<_>>(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn execute_centrality(&self, args: &Value) -> Result<String, String> {
        let block_str = args.get("block_id").and_then(|v| v.as_str());
        let top_n = args
            .get("top_n")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        // Get all blocks to build the graph
        let blocks = self
            .block_use_cases
            .get_all_blocks()
            .await
            .map_err(|e| e.to_string())?;

        let structure_map = self
            .structural_mirror
            .analyze_blocks(&blocks)
            .await;

        // Sort influences by score descending and take top_n
        let mut influences = structure_map.influences;
        influences.sort_by(|a, b| {
            b.influence_score
                .partial_cmp(&a.influence_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let scores_json: Vec<serde_json::Value> = influences
            .iter()
            .take(top_n)
            .map(|i| {
                serde_json::json!({
                    "block_id": i.block_id.to_string(),
                    "centrality": i.influence_score,
                })
            })
            .collect();

        // If block_id was provided, also return its specific score
        let block_score: Option<serde_json::Value> = block_str.and_then(|s| {
            let block_id = Uuid::parse_str(s)?;
            influences
                .iter()
                .find(|i| i.block_id == block_id)
                .map(|i| {
                    serde_json::json!({
                        "block_id": i.block_id.to_string(),
                        "centrality": i.influence_score,
                    })
                })
        });

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "scores": scores_json,
            "total_nodes": influences.len(),
            "block_score": block_score,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn execute_structure(&self, args: &Value) -> Result<String, String> {
        let page_id_str = args
            .get("page_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'page_id' parameter")?;
        let page_id =
            Uuid::parse_str(page_id_str).ok_or("Invalid page_id: must be a UUID")?;

        let detect_fallacies = args
            .get("detect_fallacies")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let block_id_str = args.get("block_id").and_then(|v| v.as_str());

        // If block_id is provided, analyze that single block's strength
        if let Some(block_str) = block_id_str {
            let block_id =
                Uuid::parse_str(block_str).ok_or("Invalid block_id: must be a UUID")?;

            let strength = self
                .structure_mapper
                .score_argument_strength(block_id)
                .await
                .map_err(|e| e.to_string())?;

            let mut result = serde_json::json!({
                "block_id": block_id.to_string(),
                "argument_strength": strength,
            });

            if detect_fallacies {
                let fallacies = self
                    .structure_mapper
                    .detect_fallacies(block_id)
                    .await
                    .map_err(|e| e.to_string())?;
                result["fallacies"] = serde_json::json!({
                    "count": fallacies.len(),
                    "detected": fallacies.iter().map(|f| {
                        serde_json::json!({
                            "fallacy_type": format!("{:?}", f.fallacy_type).to_lowercase(),
                            "block_id": f.block_id.to_string(),
                            "explanation": f.explanation,
                        })
                    }).collect::<Vec<_>>(),
                });
            }

            return Ok(serde_json::to_string_pretty(&result)
                .unwrap_or_else(|e| format!("Serialization error: {}", e)));
        }

        // Analyze entire page structure
        let graph = self
            .structure_mapper
            .map_arguments(page_id)
            .await
            .map_err(|e| e.to_string())?;

        let nodes_json: Vec<serde_json::Value> = graph
            .nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "block_id": n.block_id.to_string(),
                    "role": format!("{:?}", n.role).to_lowercase(),
                    "strength": n.strength,
                    "position": format!("{:?}", n.position).to_lowercase(),
                })
            })
            .collect();

        let edges_json: Vec<serde_json::Value> = graph
            .edges
            .iter()
            .map(|e| {
                serde_json::json!({
                    "source": e.source.to_string(),
                    "target": e.target.to_string(),
                    "edge_type": format!("{:?}", e.edge_type).to_lowercase(),
                    "confidence": e.confidence,
                })
            })
            .collect();

        let consensus_json: Vec<serde_json::Value> = graph
            .consensus_zones
            .iter()
            .map(|z| {
                serde_json::json!({
                    "block_ids": z.block_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
                    "coherence_score": z.coherence_score,
                })
            })
            .collect();

        let mut result = serde_json::json!({
            "page_id": page_id.to_string(),
            "node_count": nodes_json.len(),
            "nodes": nodes_json,
            "edge_count": edges_json.len(),
            "edges": edges_json,
            "consensus_zones": consensus_json,
        });

        if detect_fallacies {
            // Detect fallacies for all claim-type nodes
            let mut all_fallacies = Vec::new();
            for node in &graph.nodes {
                if let Ok(fallacies) = self.structure_mapper.detect_fallacies(node.block_id).await {
                    all_fallacies.extend(fallacies);
                }
            }
            result["fallacies"] = serde_json::json!({
                "count": all_fallacies.len(),
                "detected": all_fallacies.iter().map(|f| {
                    serde_json::json!({
                        "fallacy_type": format!("{:?}", f.fallacy_type).to_lowercase(),
                        "block_id": f.block_id.to_string(),
                        "explanation": f.explanation,
                    })
                }).collect::<Vec<_>>(),
            });
        }

        Ok(serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn execute_garden_health(&self, args: &Value) -> Result<String, String> {
        let agent_id = args
            .get("agent_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let depth_threshold = args
            .get("depth_threshold")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;

        // Detect contradictions
        let contradictions = self
            .structure_gardener
            .detect_contradictions(agent_id)
            .await
            .map_err(|e| e.to_string())?;

        // Get deepening suggestions
        let suggestions = self
            .structure_gardener
            .suggest_deepening(agent_id, depth_threshold)
            .await
            .map_err(|e| e.to_string())?;

        // Build mental model
        let mental_model = self
            .structure_gardener
            .build_model(agent_id)
            .await
            .map_err(|e| e.to_string())?;

        let beliefs_json: Vec<serde_json::Value> = mental_model
            .beliefs
            .iter()
            .map(|b| {
                serde_json::json!({
                    "id": b.id.to_string(),
                    "statement": b.statement,
                    "confidence": b.confidence,
                    "source_blocks": b.source_blocks.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
                    "last_updated": b.last_updated.to_rfc3339(),
                })
            })
            .collect();

        let contradictions_json: Vec<serde_json::Value> = contradictions
            .iter()
            .map(|c| {
                serde_json::json!({
                    "belief_a": c.belief_a.to_string(),
                    "belief_b": c.belief_b.to_string(),
                    "explanation": c.explanation,
                    "severity": c.severity,
                })
            })
            .collect();

        let suggestions_json: Vec<serde_json::Value> = suggestions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "concept": s.concept,
                    "current_depth": s.current_depth,
                    "suggested_questions": s.suggested_questions,
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "agent_id": agent_id,
            "belief_count": beliefs_json.len(),
            "beliefs": beliefs_json,
            "contradiction_count": contradictions_json.len(),
            "contradictions": contradictions_json,
            "deepening_suggestions": suggestions_json,
            "structure_health": if contradictions.is_empty() && suggestions.is_empty() {
                "healthy"
            } else if contradictions.len() > suggestions.len() {
                "needs_attention"
            } else {
                "can_improve"
            },
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }
}
