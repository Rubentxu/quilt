//! `decay-annotator` — the V1 built-in agent type.
//!
//! Reuses the existing `DecayMonitorService` (CG-7) to find
//! blocks that have decayed beyond the medium threshold and
//! creates an `agent-run` block on each stale page. The
//! block is the durable record of the run; the in-memory
//! `AgentLifecycle` is the live status.
//!
//! The executor is cooperative: it checks the cancel
//! `watch::Receiver` between pages and stops early if the
//! user cancels. Already-created annotation blocks are
//! kept (they're useful even if the rest of the graph
//! wasn't annotated).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::watch;

use quilt_domain::entities::Block;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};

use crate::decay_monitor::DecayMonitorService;

use super::super::registry::{AgentError, AgentExecutor, AgentRunOutcome, RunContext};

/// The V1 decay-annotator executor. Stateless — each `run()`
/// call rebuilds the per-run state from the supplied
/// `RunContext`.
pub struct DecayAnnotatorExecutor;

impl DecayAnnotatorExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DecayAnnotatorExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentExecutor for DecayAnnotatorExecutor {
    fn agent_type(&self) -> &'static str {
        "decay-annotator"
    }

    async fn run(
        &self,
        ctx: RunContext,
        cancel: watch::Receiver<bool>,
    ) -> Result<AgentRunOutcome, AgentError> {
        // 1. Detect decay alerts using the same algorithm
        //    the Decay Monitor panel uses.
        let service = DecayMonitorService::new(ctx.block_repo.clone(), ctx.page_repo.clone());
        let dto = service.detect_now().await;
        let alerts = dto.alerts;

        if alerts.is_empty() {
            return Ok(AgentRunOutcome {
                summary: if let Some(p) = &ctx.context_page {
                    format!("No decay alerts on page '{p}'")
                } else {
                    "No decay alerts found".to_string()
                },
                blocks_modified: 0,
            });
        }

        // 2. Group by page. (V1: in-memory map; the cap is
        //    already 10 alerts at the service level so this
        //    is bounded.)
        let mut by_page: std::collections::BTreeMap<
            String,
            Vec<&crate::morning_briefing::DecayAlert>,
        > = std::collections::BTreeMap::new();
        for alert in &alerts {
            by_page
                .entry(alert.page_name.clone())
                .or_default()
                .push(alert);
        }

        // 3. If a context page is set, restrict to it.
        let page_filter: Option<String> = ctx.context_page.clone();

        // 4. Iterate pages; for each, write one annotation
        //    block. Check the cancel signal between pages.
        let mut blocks_written: u32 = 0;
        let mut pages_annotated: u32 = 0;
        for (page_name, page_alerts) in &by_page {
            if let Some(filter) = &page_filter {
                if filter != page_name {
                    continue;
                }
            }
            if *cancel.borrow() {
                break;
            }
            // Idempotency: skip a page if a recent
            // (≤ 24h) decay-annotator block already exists.
            // Resolve / create the target page first so we
            // can compare on `page_id`.
            let page_id = match resolve_or_create_page(&ctx.page_repo, page_name).await {
                Ok(id) => id,
                Err(e) => {
                    return Err(AgentError::Repository(e.to_string()));
                }
            };
            if has_recent_annotation(&ctx.block_repo, page_id)
                .await
                .unwrap_or(false)
            {
                continue;
            }
            // Build the annotation block.
            let block = build_annotation_block(
                ctx.run_id,
                page_id,
                page_name,
                page_alerts,
                ctx.model.as_deref(),
            );
            if let Err(e) = ctx.block_repo.insert(&block).await {
                return Err(AgentError::Repository(e.to_string()));
            }
            blocks_written += 1;
            pages_annotated += 1;
        }

        let summary = if pages_annotated == 0 {
            if let Some(p) = &ctx.context_page {
                format!("No decay alerts on page '{p}'")
            } else {
                "No new pages needed annotation".to_string()
            }
        } else {
            format!(
                "Annotated {pages_annotated} page(s) with {} decay alert(s)",
                alerts.len()
            )
        };

        Ok(AgentRunOutcome {
            summary,
            blocks_modified: blocks_written,
        })
    }
}

/// Resolve a page name to a `Uuid`; create it on demand.
async fn resolve_or_create_page(
    page_repo: &Arc<dyn PageRepository>,
    name: &str,
) -> Result<Uuid, String> {
    if let Ok(Some(p)) = page_repo.get_by_name(name).await {
        return Ok(p.id);
    }
    use quilt_domain::entities::{Page, PageCreate};
    let create = PageCreate {
        name: name.to_string(),
        title: Some(name.to_string()),
        namespace_id: None,
        journal_day: None,
        format: quilt_domain::value_objects::BlockFormat::Markdown,
        file_id: None,
        properties: std::collections::HashMap::new(),
        // Agent-created pages are not ingested from a file
        source_path: None,
        source_mtime: None,
    };
    match Page::new(create) {
        Ok(page) => {
            if page_repo.insert(&page).await.is_err() {
                if let Ok(Some(p)) = page_repo.get_by_name(name).await {
                    return Ok(p.id);
                }
                return Err(format!("failed to create page '{name}'"));
            }
            Ok(page.id)
        }
        Err(_) => {
            if let Ok(Some(p)) = page_repo.get_by_name(name).await {
                return Ok(p.id);
            }
            Err(format!("failed to build page '{name}'"))
        }
    }
}

/// True if a `decay-annotator` block on this page was
/// created in the last 24 hours.
async fn has_recent_annotation(
    block_repo: &Arc<dyn BlockRepository>,
    page_id: Uuid,
) -> Result<bool, String> {
    // Use the dedicated `list_by_property` repository
    // method to find recent decay-annotator blocks. The
    // limit is small (20) because idempotency is a per-run
    // check, not a graph-wide scan.
    let blocks = block_repo
        .list_by_property("agent", "decay-annotator", 20)
        .await
        .map_err(|e| e.to_string())?;
    let cutoff = Utc::now() - chrono::Duration::hours(24);
    for b in blocks {
        if b.page_id != page_id {
            continue;
        }
        if b.created_at >= cutoff {
            return Ok(true);
        }
    }
    Ok(false)
}

fn build_annotation_block(
    run_id: Uuid,
    page_id: Uuid,
    page_name: &str,
    alerts: &[&crate::morning_briefing::DecayAlert],
    model: Option<&str>,
) -> Block {
    let now = Utc::now();
    let mut props: std::collections::HashMap<String, PropertyValue> =
        std::collections::HashMap::new();
    props.insert("type".into(), PropertyValue::string("agent-run"));
    props.insert("agent".into(), PropertyValue::string("decay-annotator"));
    if let Some(m) = model {
        props.insert("model".into(), PropertyValue::string(m.to_string()));
    } else {
        props.insert("model".into(), PropertyValue::string("decay-annotator-v1"));
    }
    props.insert("run-status".into(), PropertyValue::string("Completed"));
    props.insert("started-at".into(), PropertyValue::string(now.to_rfc3339()));
    props.insert(
        "completed-at".into(),
        PropertyValue::string(now.to_rfc3339()),
    );
    props.insert(
        "summary".into(),
        PropertyValue::string(format!("Decay: {} stale block(s)", alerts.len())),
    );
    props.insert(
        "agent-run-id".into(),
        PropertyValue::string(run_id.to_string()),
    );

    // Bulleted list of alerts.
    let mut content = format!(
        "🤖 Decay Annotator — {} alert(s) on [[{page_name}]]\n",
        alerts.len()
    );
    for a in alerts {
        content.push_str(&format!(
            "- (({})) — {} day(s) stale — {}\n",
            a.block_id, a.days_since_update, a.severity
        ));
    }

    Block {
        id: Uuid::new_v4(),
        page_id,
        parent_id: None,
        order: 0.0,
        level: 1,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        marker: None,
        priority: None,
        content: content.trim_end().to_string(),
        properties: props,
        refs: Vec::new(),
        tags: Vec::new(),
        scheduled: None,
        deadline: None,
        start_time: None,
        repeated: None,
        logbook: None,
        completed_at: Some(now),
        cancelled_at: None,
        collapsed: false,
        created_at: now,
        updated_at: now,
    }
}

#[cfg(test)]
mod tests_disabled {
    // Tests moved to `tests/agent_room_integration.rs`.
}
