//! Cognitive handler implementation for MCP tools.
//!
//! Implements [`CognitiveHandler`] trait by delegating to the various
//! cognitive engine instances (CognitiveMirror, SerendipityEngine, etc.).

use super::{CognitiveHandler, HandlerResult};
use async_trait::async_trait;
use quilt_cognitive::tree_rag::ReportScope;
use quilt_cognitive::{
    AgentMemory, ArgumentCartographer, CognitiveMirror, CounterfactualExplorer,
    KnowledgeEvolutionTracker, MentalModelGardener, MorningBriefing, SerendipityEngine,
    TaskScheduler, TreeRagEngine,
};
use quilt_domain::value_objects::Uuid;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

// ── Local wire-format DTOs ────────────────────────────────────────
//
// These mirror the existing `serde_json::json!({ ... })` shapes used
// by the handler responses. The DTOs are local to this module —
// they isolate the wire format from the engine return types so adding
// fields to either side does not silently change the other.

/// Wire shape for a single cluster in `explore_topic`.
#[derive(Serialize)]
struct ExploreClusterWire {
    label: String,
    summary: String,
    block_ids: Vec<String>,
    relevance: f32,
}

/// Wire shape for the `explore_topic` outer response.
#[derive(Serialize)]
struct ExploreTopicResponse {
    topic: String,
    cluster_count: usize,
    total_blocks: usize,
    clusters: Vec<ExploreClusterWire>,
}

/// Wire shape for `build_tree` and `query_tree` (identical envelope).
#[derive(Serialize)]
struct TreeEnvelope {
    page_id: String,
    page_name: String,
    total_blocks: usize,
    root: Value,
}

/// Wire shape for `assemble_report`.
#[derive(Serialize)]
struct AssembleReportResponse {
    title: String,
    description: String,
    markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pdf_size_bytes: Option<usize>,
    citations_count: usize,
}

/// Wire shape for `tree_status` — `coverage_percent` is derived.
#[derive(Serialize)]
struct TreeStatusResponse {
    total_blocks: u64,
    indexed_blocks: u64,
    pending_blocks: u64,
    coverage_percent: u32,
}

/// Wire shape for `save_block_summary` and `rebuild_tree_index`
/// (small status envelopes).
#[derive(Serialize)]
struct SaveBlockSummaryResponse {
    status: &'static str,
    block_id: String,
}

#[derive(Serialize)]
struct RebuildStartedResponse {
    status: &'static str,
}

/// Wire shape for `schedule_task`.
#[derive(Serialize)]
struct ScheduleTaskResponse<'a> {
    scheduled: &'a str,
    cron: &'a str,
}

/// Wire shape for one task in `list_tasks` (dates formatted to RFC3339).
#[derive(Serialize)]
struct ScheduledTaskWire {
    name: String,
    cron_expr: String,
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_run: Option<String>,
    next_run: String,
}

/// Wire shape for `resource_cognitive_map`.
#[derive(Serialize)]
struct CognitiveMapSummary {
    total_clusters: usize,
    total_frontiers: usize,
    total_gaps: usize,
    pages_analyzed: usize,
}

/// Default implementation of [`CognitiveHandler`] that wraps all cognitive engines.
///
/// All engines are optional - methods will return an error if the engine
/// they need is not configured.
#[derive(Clone)]
pub struct DefaultCognitiveHandler {
    /// Repository for page lookups by name.
    page_repo: Arc<dyn quilt_domain::repositories::PageRepository>,
    /// Cognitive mirror engine.
    cognitive_mirror: Option<Arc<CognitiveMirror>>,
    /// Serendipity engine.
    serendipity_engine: Option<Arc<SerendipityEngine>>,
    /// Agent memory engine.
    agent_memory: Option<Arc<AgentMemory>>,
    /// Argument cartographer.
    argument_cartographer: Option<Arc<ArgumentCartographer>>,
    /// Mental model gardener.
    mental_model_gardener: Option<Arc<MentalModelGardener>>,
    /// Counterfactual explorer.
    counterfactual_explorer: Option<Arc<CounterfactualExplorer>>,
    /// Knowledge evolution tracker.
    knowledge_evolution_tracker: Option<Arc<KnowledgeEvolutionTracker>>,
    /// Morning briefing service.
    morning_briefing: Option<Arc<MorningBriefing>>,
    /// Tree RAG engine.
    tree_rag: Option<Arc<TreeRagEngine>>,
    /// Task scheduler.
    task_scheduler: Option<Arc<TaskScheduler>>,
}

impl DefaultCognitiveHandler {
    /// Create a new cognitive handler with the given dependencies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        page_repo: Arc<dyn quilt_domain::repositories::PageRepository>,
        cognitive_mirror: Option<Arc<CognitiveMirror>>,
        serendipity_engine: Option<Arc<SerendipityEngine>>,
        agent_memory: Option<Arc<AgentMemory>>,
        argument_cartographer: Option<Arc<ArgumentCartographer>>,
        mental_model_gardener: Option<Arc<MentalModelGardener>>,
        counterfactual_explorer: Option<Arc<CounterfactualExplorer>>,
        knowledge_evolution_tracker: Option<Arc<KnowledgeEvolutionTracker>>,
        morning_briefing: Option<Arc<MorningBriefing>>,
        tree_rag: Option<Arc<TreeRagEngine>>,
        task_scheduler: Option<Arc<TaskScheduler>>,
    ) -> Self {
        Self {
            page_repo,
            cognitive_mirror,
            serendipity_engine,
            agent_memory,
            argument_cartographer,
            mental_model_gardener,
            counterfactual_explorer,
            knowledge_evolution_tracker,
            morning_briefing,
            tree_rag,
            task_scheduler,
        }
    }

    /// Parse a scope string into a [`ReportScope`].
    fn parse_scope(s: &str) -> Result<ReportScope, String> {
        Ok(match s {
            "auto" => ReportScope::Auto,
            "all" => ReportScope::AllPages,
            s if s.starts_with("pages:") => {
                let names: Vec<String> = s
                    .strip_prefix("pages:")
                    .unwrap()
                    .split(',')
                    .map(|n| n.trim().to_string())
                    .collect();
                ReportScope::Pages(names)
            }
            s if s.starts_with("journal:") => {
                let days: u32 = s
                    .strip_prefix("journal:")
                    .unwrap()
                    .parse()
                    .map_err(|_| "Invalid journal days")?;
                ReportScope::JournalLast(days)
            }
            s if s.starts_with("tagged:") => {
                let tag = s.strip_prefix("tagged:").unwrap().to_string();
                ReportScope::Tagged(tag)
            }
            other => {
                return Err(format!(
                    "Unknown scope: {}. Use auto, all, pages:name1,name2, journal:N, or tagged:tag",
                    other
                ))
            }
        })
    }

    /// Find a page by name.
    async fn find_page_by_name(
        &self,
        page_name: &str,
    ) -> Result<quilt_domain::entities::Page, String> {
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        pages
            .iter()
            .find(|p| p.name == page_name)
            .cloned()
            .ok_or_else(|| format!("Page not found: {}", page_name))
    }
}

#[async_trait]
impl CognitiveHandler for DefaultCognitiveHandler {
    #[instrument(skip(self))]
    async fn cognitive_mirror(&self, page_name: &str) -> HandlerResult {
        let mirror = self
            .cognitive_mirror
            .as_ref()
            .ok_or_else(|| "CognitiveMirror not configured".to_string())?;

        let page = self.find_page_by_name(page_name).await?;
        let map = mirror.analyze(page.id).await.map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&map).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn serendipity(
        &self,
        since: Option<String>,
        limit: Option<usize>,
        min_confidence: Option<f32>,
    ) -> HandlerResult {
        let since = since
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let limit = limit.unwrap_or(20);
        let min_confidence = min_confidence.unwrap_or(0.3);

        let engine = self
            .serendipity_engine
            .as_ref()
            .ok_or_else(|| "SerendipityEngine not configured".to_string())?;

        let since_utc = since.unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::days(7));
        let since_days_ago = (chrono::Utc::now() - since_utc).num_days();
        let query = quilt_cognitive::serendipity::SerendipityQuery {
            topic: None,
            limit,
            offset: 0,
            min_confidence,
            temporal_window_days: Some(since_days_ago),
            page_id: None,
        };

        let connections = engine
            .find_connections(query)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&connections).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn agent_memory(
        &self,
        domain: &str,
        query: Option<&str>,
        limit: Option<usize>,
    ) -> HandlerResult {
        let limit = limit.unwrap_or(10);

        let memory = self
            .agent_memory
            .as_ref()
            .ok_or_else(|| "AgentMemory not configured".to_string())?;

        let mem_query = quilt_cognitive::agent_memory::MemoryQuery {
            agent_id: domain.to_string(),
            context: None,
            query: query.map(String::from),
            limit,
        };

        let entries = memory
            .retrieve(mem_query)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&entries).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn argument_map(&self, page_name: &str, _max_depth: Option<usize>) -> HandlerResult {
        let cartographer = self
            .argument_cartographer
            .as_ref()
            .ok_or_else(|| "ArgumentCartographer not configured".to_string())?;

        let page = self.find_page_by_name(page_name).await?;
        let graph = cartographer
            .map_arguments(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&graph).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn mental_model(&self, agent_id: &str, time_window: Option<String>) -> HandlerResult {
        let _time_window = time_window
            .and_then(|s| s.parse::<i64>().ok())
            .map(|days| chrono::Duration::days(days));

        let gardener = self
            .mental_model_gardener
            .as_ref()
            .ok_or_else(|| "MentalModelGardener not configured".to_string())?;

        let model = gardener
            .build_model(agent_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&model).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn counterfactual_explore(&self, scenario: &str, decision_point: &str) -> HandlerResult {
        let explorer = self
            .counterfactual_explorer
            .as_ref()
            .ok_or_else(|| "CounterfactualExplorer not configured".to_string())?;

        let tree = explorer
            .explore(scenario, decision_point)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&tree).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn knowledge_evolution(
        &self,
        topic: &str,
        timespan_days: Option<usize>,
    ) -> HandlerResult {
        let timespan_days = timespan_days.unwrap_or(30) as u32;

        let tracker = self
            .knowledge_evolution_tracker
            .as_ref()
            .ok_or_else(|| "KnowledgeEvolutionTracker not configured".to_string())?;

        let timeline = tracker
            .track(topic, timespan_days)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&timeline).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn morning_briefing(&self) -> HandlerResult {
        let briefing = self
            .morning_briefing
            .as_ref()
            .ok_or_else(|| "MorningBriefing not configured".to_string())?;

        let dto = briefing.generate().await;
        serde_json::to_string_pretty(&dto).map_err(|e| e.to_string())
    }

    #[instrument(skip(self))]
    async fn explore_topic(&self, topic: &str, scope: Option<String>) -> HandlerResult {
        let scope_str = scope.as_deref().unwrap_or("auto");

        let engine = self
            .tree_rag
            .as_ref()
            .ok_or_else(|| "TreeRAG not configured".to_string())?;

        let scope = Self::parse_scope(scope_str)?;
        let clusters = engine
            .explore_topic(topic, &scope)
            .await
            .map_err(|e| e.to_string())?;

        let cluster_wires: Vec<ExploreClusterWire> = clusters
            .iter()
            .map(|c| ExploreClusterWire {
                label: c.label.clone(),
                summary: c.summary.clone(),
                block_ids: c.block_ids.iter().map(|id| id.to_string()).collect(),
                relevance: c.relevance,
            })
            .collect();

        let total_blocks: usize = clusters.iter().map(|c| c.block_ids.len()).sum();
        let response = ExploreTopicResponse {
            topic: topic.to_string(),
            cluster_count: clusters.len(),
            total_blocks,
            clusters: cluster_wires,
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn build_tree(&self, page_id: Uuid) -> HandlerResult {
        let engine = self
            .tree_rag
            .as_ref()
            .ok_or_else(|| "TreeRAG not configured".to_string())?;

        let tree = engine
            .build_tree(page_id)
            .await
            .map_err(|e| e.to_string())?;

        let response = TreeEnvelope {
            page_id: tree.page_id.to_string(),
            page_name: tree.page_name,
            total_blocks: tree.total_blocks,
            root: tree.root,
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn query_tree(&self, page_id: Uuid, query: &str) -> HandlerResult {
        let engine = self
            .tree_rag
            .as_ref()
            .ok_or_else(|| "TreeRAG not configured".to_string())?;

        let tree = engine
            .query_tree(page_id, query)
            .await
            .map_err(|e| e.to_string())?;

        let response = TreeEnvelope {
            page_id: tree.page_id.to_string(),
            page_name: tree.page_name,
            total_blocks: tree.total_blocks,
            root: tree.root,
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn assemble_report(
        &self,
        title: &str,
        description: &str,
        sections: serde_json::Value,
        render_pdf: bool,
    ) -> HandlerResult {
        let sections = sections
            .as_array()
            .ok_or_else(|| "sections must be an array".to_string())?;

        let engine = self
            .tree_rag
            .as_ref()
            .ok_or_else(|| "TreeRAG not configured".to_string())?;

        let assembled_sections: Vec<quilt_cognitive::tree_rag::AssembledSection> = sections
            .iter()
            .filter_map(|s| {
                let heading = s.get("heading")?.as_str()?.to_string();
                let level = s.get("level")?.as_u64()? as u8;
                let content = s.get("content")?.as_str()?.to_string();
                let source_block_ids: Vec<Uuid> = s
                    .get("source_block_ids")
                    .and_then(|arr| arr.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(Uuid::parse_str)
                            .collect()
                    })
                    .unwrap_or_default();
                let subsections: Vec<quilt_cognitive::tree_rag::AssembledSection> = s
                    .get("subsections")
                    .and_then(|arr| arr.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|sub| {
                                let heading = sub.get("heading")?.as_str()?.to_string();
                                let level = sub.get("level")?.as_u64()? as u8;
                                let content = sub.get("content")?.as_str()?.to_string();
                                let source_block_ids: Vec<Uuid> = sub
                                    .get("source_block_ids")
                                    .and_then(|arr| arr.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str())
                                            .filter_map(Uuid::parse_str)
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                Some(quilt_cognitive::tree_rag::AssembledSection {
                                    heading,
                                    level,
                                    content,
                                    source_block_ids,
                                    subsections: vec![],
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                Some(quilt_cognitive::tree_rag::AssembledSection {
                    heading,
                    level,
                    content,
                    source_block_ids,
                    subsections,
                })
            })
            .collect();

        let markdown = engine.assemble_document(title, description, &assembled_sections);

        let pdf_bytes = if render_pdf {
            Some(engine.render_pdf(&markdown).map_err(|e| e.to_string())?)
        } else {
            None
        };

        let citations = assembled_sections
            .iter()
            .flat_map(|s| s.source_block_ids.iter())
            .collect::<Vec<_>>();

        let response = AssembleReportResponse {
            title: title.to_string(),
            description: description.to_string(),
            markdown,
            pdf_size_bytes: pdf_bytes.as_ref().map(|b| b.len()),
            citations_count: citations.len(),
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn tree_status(&self) -> HandlerResult {
        let engine = self
            .tree_rag
            .as_ref()
            .ok_or_else(|| "TreeRAG not configured".to_string())?;

        let status = engine.status().await.map_err(|e| e.to_string())?;

        let coverage_percent = if status.total_blocks > 0 {
            (status.indexed_blocks as f64 / status.total_blocks as f64 * 100.0) as u32
        } else {
            0
        };

        let response = TreeStatusResponse {
            total_blocks: status.total_blocks,
            indexed_blocks: status.indexed_blocks,
            pending_blocks: status.pending_blocks,
            coverage_percent,
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn save_block_summary(&self, block_id: Uuid, summary: &str) -> HandlerResult {
        let engine = self
            .tree_rag
            .as_ref()
            .ok_or_else(|| "TreeRAG not configured".to_string())?;

        engine
            .save_block_summary(block_id, summary.to_string())
            .await
            .map_err(|e| e.to_string())?;

        let response = SaveBlockSummaryResponse {
            status: "saved",
            block_id: block_id.to_string(),
        };
        serde_json::to_string(&response).map_err(|e| e.to_string())
    }

    #[instrument(skip(self))]
    async fn rebuild_tree_index(&self, scope: Option<String>) -> HandlerResult {
        let engine = self
            .tree_rag
            .as_ref()
            .ok_or_else(|| "TreeRAG not configured".to_string())?;

        // Parse scope string into ReportScope
        let scope = scope.map(|s| Self::parse_scope(&s)).transpose()?;

        // Note: rebuild_index is async but doesn't return a meaningful result
        engine
            .rebuild_index(scope.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let response = RebuildStartedResponse {
            status: "rebuild_started",
        };
        serde_json::to_string(&response).map_err(|e| e.to_string())
    }

    #[instrument(skip(self))]
    async fn schedule_task(&self, name: &str, cron_expr: &str, task_type: &str) -> HandlerResult {
        let scheduler = self
            .task_scheduler
            .as_ref()
            .ok_or_else(|| "TaskScheduler not configured".to_string())?;

        let task_type = match task_type {
            "RebuildIndex" => quilt_domain::TaskType::RebuildIndex,
            "CleanStaleSummaries" => quilt_domain::TaskType::CleanStaleSummaries,
            "HealthCheck" => quilt_domain::TaskType::HealthCheck,
            other => return Err(format!("Unknown task_type: {}", other)),
        };

        scheduler
            .schedule_task(name, cron_expr, task_type)
            .await
            .map_err(|e| e.to_string())?;

        let response = ScheduleTaskResponse {
            scheduled: name,
            cron: cron_expr,
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn list_tasks(&self) -> HandlerResult {
        let scheduler = self
            .task_scheduler
            .as_ref()
            .ok_or_else(|| "TaskScheduler not configured".to_string())?;

        let tasks = scheduler.list_tasks().await?;

        let json_tasks: Vec<ScheduledTaskWire> = tasks
            .iter()
            .map(|t| ScheduledTaskWire {
                name: t.name.clone(),
                cron_expr: t.cron_expr.clone(),
                enabled: t.enabled,
                last_run: t.last_run.map(|d| d.to_rfc3339()),
                next_run: t.next_run.to_rfc3339(),
            })
            .collect();

        Ok(serde_json::to_string_pretty(&json_tasks).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn resource_cognitive_map(&self) -> HandlerResult {
        let mirror = self
            .cognitive_mirror
            .as_ref()
            .ok_or_else(|| "CognitiveMirror not configured".to_string())?;

        // Get overall stats by analyzing recent pages
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let recent_pages: Vec<_> = pages.iter().take(10).collect();

        let mut total_clusters = 0;
        let mut total_frontiers = 0;
        let mut total_gaps = 0;
        let pages_count = recent_pages.len();

        for page in &recent_pages {
            if let Ok(map) = mirror.analyze(page.id).await {
                total_clusters += map.clusters.len();
                total_frontiers += map.frontiers.len();
                total_gaps += map.gaps.len();
            }
        }

        let response = CognitiveMapSummary {
            total_clusters,
            total_frontiers,
            total_gaps,
            pages_analyzed: pages_count,
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn resource_cognitive_serendipity(&self) -> HandlerResult {
        let engine = self
            .serendipity_engine
            .as_ref()
            .ok_or_else(|| "SerendipityEngine not configured".to_string())?;

        let query = quilt_cognitive::serendipity::SerendipityQuery {
            topic: None,
            limit: 20,
            offset: 0,
            min_confidence: 0.3,
            temporal_window_days: Some(30),
            page_id: None,
        };

        let connections = engine
            .find_connections(query)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&connections).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn resource_arguments(&self, uri: &str) -> HandlerResult {
        let page_name = uri
            .strip_prefix("quilt://cognitive/arguments/")
            .ok_or_else(|| "Invalid arguments resource URI".to_string())?;

        let cartographer = self
            .argument_cartographer
            .as_ref()
            .ok_or_else(|| "ArgumentCartographer not configured".to_string())?;

        // Find page by name
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let page = pages
            .iter()
            .find(|p| p.name == page_name)
            .ok_or_else(|| format!("No arguments found for page: {}", page_name))?;

        let graph = cartographer
            .map_arguments(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&graph).unwrap_or_else(|e| e.to_string()))
    }

    #[instrument(skip(self))]
    async fn resource_mental_models(&self) -> HandlerResult {
        let gardener = self
            .mental_model_gardener
            .as_ref()
            .ok_or_else(|| "MentalModelGardener not configured".to_string())?;

        // Get all journals as potential agents
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let journals: Vec<_> = pages.iter().filter(|p| p.journal).collect();

        let mut models = Vec::new();
        for journal in journals.iter().take(10) {
            if let Ok(model) = gardener.build_model(&journal.name).await {
                models.push(model);
            }
        }

        Ok(serde_json::to_string_pretty(&models).unwrap_or_else(|e| e.to_string()))
    }
}
