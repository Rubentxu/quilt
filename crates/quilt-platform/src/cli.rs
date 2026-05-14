//! CLI implementation for Quilt

use anyhow::Result;
use clap::{Parser, Subcommand};
use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use quilt_cognitive::{
    AgentMemory, ArgumentCartographer, CognitiveMirror, CounterfactualExplorer,
    KnowledgeEvolutionTracker, MentalModelGardener, MockAIClient, SerendipityEngine,
    TaskScheduler, TreeRagEngine,
};
use quilt_domain::entities::{BlockCreate, PageCreate, UserSettings};
use quilt_domain::repositories::{BlockRepository, PageRepository, SettingsRepository};
use quilt_domain::services::TimezoneService;
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqliteBlockSummaryRepository, SqliteDeepLinkRepository,
    SqliteJournalRepository, SqlitePageRepository, SqliteScheduledTaskRepository,
    SqliteSettingsRepository, SqliteTagRepository,
};
use quilt_mcp::McpServer;
use quilt_search::SearchService;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::mcp_transport::StdioTransport;

#[derive(Parser)]
#[command(name = "quilt")]
#[command(about = "Quilt - AI-first Knowledge Graph", long_about = None)]
pub struct QuiltCLI {
    #[command(subcommand)]
    pub command: Command,

    /// Path to the graph database
    #[arg(short, long, default_value = "quilt.db")]
    pub db_path: PathBuf,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new graph database
    Init {
        /// Name of the graph
        #[arg(short, long)]
        name: String,
    },
    /// Open an existing graph
    Open {
        /// Name of the graph to open
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Create a new page
    Page {
        /// Page name
        #[arg(short, long)]
        name: String,
    },
    /// Create a new block
    Block {
        /// Page name to add block to
        #[arg(short, long)]
        page: String,
        /// Block content (markdown)
        #[arg(short, long)]
        content: String,
        /// Parent block UUID (optional)
        #[arg(long)]
        parent: Option<String>,
    },
    /// Create a journal page for today
    Journal {
        /// Date in YYYY-MM-DD format (defaults to today)
        #[arg(short, long)]
        date: Option<String>,
    },
    /// Execute a query
    Query {
        /// DSL query string
        #[arg(short, long)]
        dsl: String,
        /// Max results
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
    /// Search across all content
    Search {
        /// Search query
        #[arg(short, long)]
        query: String,
    },
    /// Start the MCP server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3541")]
        port: u16,
    },
    /// List all pages
    ListPages,
    /// Get page info
    PageInfo {
        /// Page name
        #[arg(short, long)]
        name: String,
    },
    /// TreeRAG operations (explore, build tree, assemble report)
    TreeRag {
        #[command(subcommand)]
        command: TreeRagCommand,
    },
    /// Scheduler operations (list, schedule, run tasks)
    Scheduler {
        #[command(subcommand)]
        command: SchedulerCommand,
    },
}

#[derive(Subcommand)]
pub enum TreeRagCommand {
    /// Explore a topic: cluster blocks by theme
    Explore {
        /// Topic to explore
        #[arg(short, long)]
        topic: String,
        /// Scope: auto, all, pages:name1,name2, journal:N
        #[arg(short, long, default_value = "auto")]
        scope: String,
    },
    /// Build a tree index for a page
    BuildTree {
        /// Page ID (UUID)
        #[arg(short, long)]
        page_id: String,
    },
    /// Query a tree by text
    QueryTree {
        /// Page ID (UUID)
        #[arg(short, long)]
        page_id: String,
        /// Query string
        #[arg(short, long)]
        query: String,
    },
    /// Assemble a Markdown report from sections
    AssembleReport {
        /// Report title
        #[arg(short, long)]
        title: String,
        /// Report description
        #[arg(short, long)]
        description: String,
        /// Path to JSON file with sections
        #[arg(short, long)]
        sections_file: String,
        /// Render PDF output
        #[arg(long, default_value = "false")]
        render_pdf: bool,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Get index status
    Status,
    /// Rebuild the index (count stale blocks)
    RebuildIndex {
        /// Optional scope
        #[arg(short, long)]
        scope: Option<String>,
    },
    /// Save a block summary (from AI agent)
    SaveSummary {
        /// Block ID (UUID)
        #[arg(short, long)]
        block_id: String,
        /// Summary text
        #[arg(short, long)]
        summary: String,
    },
}

#[derive(Subcommand)]
pub enum SchedulerCommand {
    /// List all scheduled tasks
    List,
    /// Schedule a recurring task
    Schedule {
        /// Task name
        #[arg(short, long)]
        name: String,
        /// Cron expression
        #[arg(short, long)]
        cron: String,
        /// Task type: RebuildIndex, CleanStaleSummaries, HealthCheck
        #[arg(short, long)]
        task_type: String,
    },
    /// Run a task immediately
    RunNow {
        /// Task name
        #[arg(short, long)]
        name: String,
    },
    /// Delete a scheduled task
    Delete {
        /// Task name
        #[arg(short, long)]
        name: String,
    },
}

impl QuiltCLI {
    pub async fn run(&self) -> Result<()> {
        // Initialize metrics recorder with Prometheus exporter
        PrometheusBuilder::new()
            .install()
            .expect("Failed to install metrics exporter");

        describe_gauge!("quilt_pages_total", "Total number of pages");
        describe_gauge!("quilt_blocks_total", "Total number of blocks");
        gauge!("quilt_pages_total", 0.0);
        gauge!("quilt_blocks_total", 0.0);

        let pool = connection::create_pool(&self.db_path).await?;
        connection::run_migrations(&pool).await?;

        let block_repo = SqliteBlockRepository::new(pool.clone());
        let page_repo = SqlitePageRepository::new(pool.clone());
        let search_service = SearchService::new(pool.clone());

        match &self.command {
            Command::Init { name } => self.run_init(name)?,
            Command::Open { name } => {
                let pages = page_repo.get_all().await?;
                println!(
                    "Graph: {} (database: {})",
                    name.as_deref().unwrap_or("quilt"),
                    self.db_path.display()
                );
                println!("  Pages: {}", pages.len());
                let total_blocks: usize = {
                    let mut count = 0;
                    for p in &pages {
                        count += block_repo.count_by_page(p.id).await.unwrap_or(0);
                    }
                    count
                };
                println!("  Blocks: {}", total_blocks);
            }
            Command::Page { name } => self.run_page(&page_repo, name).await?,
            Command::Block {
                page,
                content,
                parent,
            } => {
                self.run_block(&page_repo, &block_repo, page, content, parent.as_deref())
                    .await?
            }
            Command::Journal { date } => self.run_journal(&page_repo, date.as_deref()).await?,
            Command::Query { dsl, limit } => self.run_query(dsl, *limit)?,
            Command::Search { query } => self.run_search(&search_service, query).await?,
            Command::Serve { port: _ } => {
                let tag_repo = SqliteTagRepository::new(pool.clone());
                let search_svc = SearchService::new(pool.clone());
                let settings_repo = Arc::new(SqliteSettingsRepository::new(pool.clone()));
                let journal_repo = Arc::new(SqliteJournalRepository::new(pool.clone()));

                // Create timezone service from user settings (fallback to UTC)
                let user_settings = settings_repo.get_user_settings().await.unwrap_or_default();
                let timezone_service = Arc::new(
                    TimezoneService::from_tz_string(&user_settings.timezone)
                        .unwrap_or_else(|_| TimezoneService::from_tz_string("UTC").unwrap()),
                );

                // Create AI client for cognitive engines
                let ai_client: Arc<dyn quilt_cognitive::AIClient> = Arc::new(MockAIClient::new());

                // Create AgentMemory first (needed by MentalModelGardener)
                let agent_memory = Arc::new(AgentMemory::new(
                    Arc::new(block_repo.clone()),
                    ai_client.clone(),
                ));

                // Create all cognitive engines
                let cognitive_mirror = Arc::new(CognitiveMirror::new(
                    Arc::new(block_repo.clone()),
                    ai_client.clone(),
                ));
                let serendipity_engine = Arc::new(SerendipityEngine::new(
                    Arc::new(block_repo.clone()),
                    ai_client.clone(),
                ));
                let argument_cartographer = Arc::new(ArgumentCartographer::new(
                    Arc::new(block_repo.clone()),
                    ai_client.clone(),
                ));
                let counterfactual_explorer = Arc::new(CounterfactualExplorer::new(
                    Arc::new(block_repo.clone()),
                    ai_client.clone(),
                ));
                let knowledge_evolution_tracker = Arc::new(KnowledgeEvolutionTracker::new(
                    Arc::new(block_repo.clone()),
                    ai_client.clone(),
                ));
                let mental_model_gardener = Arc::new(MentalModelGardener::new(
                    Arc::new(block_repo.clone()),
                    agent_memory.clone(),
                    ai_client.clone(),
                ));

                let deep_link_repo = SqliteDeepLinkRepository::new(pool.clone());

                let mcp_server = Arc::new(
                    McpServer::new(
                        Arc::new(block_repo),
                        Arc::new(page_repo),
                        Arc::new(tag_repo),
                        Arc::new(deep_link_repo),
                        Arc::new(search_svc),
                        timezone_service,
                    )
                    .with_cognitive(
                        Some(cognitive_mirror),
                        Some(serendipity_engine),
                        Some(agent_memory),
                        Some(argument_cartographer),
                        Some(mental_model_gardener),
                        Some(counterfactual_explorer),
                        Some(knowledge_evolution_tracker),
                    )
                    .with_journal_repo(journal_repo)
                    .with_settings_repo(settings_repo),
                );
                StdioTransport::serve(mcp_server).await?;
                unreachable!("StdioTransport::serve only returns on error/EOF");
            }
            Command::ListPages => self.run_list_pages(&page_repo).await?,
            Command::PageInfo { name } => self.run_page_info(&page_repo, &block_repo, name).await?,
            Command::TreeRag { command } => {
                self.run_tree_rag(&pool, command).await?
            }
            Command::Scheduler { command } => {
                self.run_scheduler(&pool, command).await?
            }
        }
        Ok(())
    }

    // ── Command handlers ─────────────────────────────────────────

    fn run_init(&self, name: &str) -> Result<()> {
        use std::fs;
        if !self.db_path.exists() {
            fs::File::create(&self.db_path)?;
        }
        println!("✓ Graph initialized: {}", name);
        println!("  Database: {}", self.db_path.display());
        Ok(())
    }

    async fn run_page(&self, repo: &SqlitePageRepository, name: &str) -> Result<()> {
        let page = PageCreate {
            name: name.to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        };
        let page = quilt_domain::entities::Page::new(page)?;
        repo.insert(&page).await?;
        println!("✓ Page created: {}", name);
        Ok(())
    }

    async fn run_block(
        &self,
        page_repo: &SqlitePageRepository,
        block_repo: &SqliteBlockRepository,
        page_name: &str,
        content: &str,
        parent_id: Option<&str>,
    ) -> Result<()> {
        // Create a default timezone service (UTC) for CLI block creation
        let timezone = TimezoneService::from_tz_string("UTC")
            .expect("UTC is a valid timezone");

        let page = match page_repo.get_by_name(page_name).await? {
            Some(p) => p,
            None => {
                let p = quilt_domain::entities::Page::new(PageCreate {
                    name: page_name.to_string(),
                    title: None,
                    namespace_id: None,
                    journal_day: None,
                    format: BlockFormat::Markdown,
                    file_id: None,
                })?;
                page_repo.insert(&p).await?;
                p
            }
        };

        let parent_uuid = parent_id
            .map(|s| Uuid::parse_str(s).ok_or_else(|| anyhow::anyhow!("Invalid UUID: {}", s)))
            .transpose()?;

        let block = quilt_domain::entities::Block::new(BlockCreate {
            page_id: page.id,
            content: content.to_string(),
            parent_id: parent_uuid,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: Default::default(),
        }, &timezone)?;

        block_repo.insert(&block).await?;
        println!("✓ Block created on page '{}': {}", page_name, content);
        Ok(())
    }

    async fn run_journal(
        &self,
        page_repo: &SqlitePageRepository,
        date_str: Option<&str>,
    ) -> Result<()> {
        let day = match date_str {
            Some(s) => JournalDay::from_str(s)?,
            None => JournalDay::from_datetime(&chrono::Utc::now()),
        };

        let page = match page_repo.get_journal(day).await? {
            Some(p) => {
                println!("Journal exists: {} (day {})", p.name, day.as_i32());
                p
            }
            None => {
                let p = quilt_domain::entities::Page::new_journal(day, BlockFormat::Markdown)?;
                page_repo.insert(&p).await?;
                println!("✓ Journal created: {} (day {})", p.name, day.as_i32());
                p
            }
        };
        let _ = page;
        Ok(())
    }

    fn run_query(&self, dsl: &str, limit: usize) -> Result<()> {
        use quilt_application::query_service::QueryService;
        let service = QueryService::new();
        match service.prepare(dsl, limit) {
            Ok(result) => {
                println!("Query: {}", dsl);
                println!("  AST: {}", result.ast);
                println!("  SQL: {}", result.sql);
                println!("  Params: {:?}", result.params);
            }
            Err(e) => {
                anyhow::bail!("Query error: {}", e);
            }
        }
        Ok(())
    }

    async fn run_search(&self, search: &SearchService, query: &str) -> Result<()> {
        println!("Search: {}", query);
        let results = search
            .search(query, 20)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        if results.is_empty() {
            println!("No results found.");
        } else {
            println!("Found {} results:", results.len());
            for (i, r) in results.iter().enumerate() {
                println!(
                    "  {}. [{page}] {snippet}",
                    i + 1,
                    page = r.page_name,
                    snippet = r.snippet
                );
            }
        }
        Ok(())
    }

    async fn run_list_pages(&self, repo: &SqlitePageRepository) -> Result<()> {
        let pages = repo.get_all().await?;
        if pages.is_empty() {
            println!("No pages found.");
        } else {
            println!("Pages ({}):", pages.len());
            for page in &pages {
                let marker = if page.journal { " 📅" } else { "" };
                println!("  - {}{}", page.name, marker);
            }
        }
        Ok(())
    }

    async fn run_page_info(
        &self,
        page_repo: &SqlitePageRepository,
        block_repo: &SqliteBlockRepository,
        name: &str,
    ) -> Result<()> {
        let page = page_repo
            .get_by_name(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found: {}", name))?;

        let block_count = block_repo.count_by_page(page.id).await?;

        println!("Page: {}", page.name);
        println!("  ID: {}", page.id);
        if let Some(title) = &page.title {
            println!("  Title: {}", title);
        }
        if page.journal {
            println!(
                "  Type: journal (day {})",
                page.journal_day.map(|d| d.as_i32()).unwrap_or(0)
            );
        }
        println!("  Blocks: {}", block_count);
        println!("  Created: {}", page.created_at);
        println!("  Updated: {}", page.updated_at);

        // Show recent blocks
        if block_count > 0 {
            let blocks = block_repo.get_by_page(page.id).await?;
            println!("  Recent blocks:");
            for b in blocks.iter().rev().take(5) {
                let prefix: &str = if b.content.len() > 60 {
                    &b.content[..60]
                } else {
                    &b.content
                };
                println!("    - {}", prefix);
            }
        }
        Ok(())
    }

    // ── TreeRAG handlers ───────────────────────────────────────────────

    async fn run_tree_rag(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        command: &TreeRagCommand,
    ) -> Result<()> {
        let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
        let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
        let summary_repo = Arc::new(SqliteBlockSummaryRepository::new(pool.clone()));

        let engine = Arc::new(TreeRagEngine::new(
            block_repo.clone(),
            page_repo.clone(),
            summary_repo.clone(),
        ));

        match command {
            TreeRagCommand::Explore { topic, scope } => {
                let scope = self.parse_report_scope(scope)?;
                let clusters = engine.explore_topic(topic, &scope).await?;
                println!("Topic: {}", topic);
                println!("Clusters ({}):", clusters.len());
                for c in &clusters {
                    println!(
                        "  - {} (relevance: {:.2}, {} blocks)",
                        c.label, c.relevance, c.block_ids.len()
                    );
                    println!("    Summary: {}", c.summary);
                }
            }
            TreeRagCommand::BuildTree { page_id } => {
                let pid = Uuid::parse_str(page_id)
                    .ok_or_else(|| anyhow::anyhow!("Invalid UUID: {}", page_id))?;
                let tree = engine.build_tree(pid).await?;
                println!("Tree for page '{}':", tree.page_name);
                println!("  Total blocks: {}", tree.total_blocks);
                self.print_tree_node(&tree.root, 2)?;
            }
            TreeRagCommand::QueryTree { page_id, query } => {
                let pid = Uuid::parse_str(page_id)
                    .ok_or_else(|| anyhow::anyhow!("Invalid UUID: {}", page_id))?;
                let tree = engine.query_tree(pid, query).await?;
                println!("Query '{}' on page '{}':", query, tree.page_name);
                println!("  Matching blocks: {}", tree.total_blocks);
                self.print_tree_node(&tree.root, 2)?;
            }
            TreeRagCommand::AssembleReport {
                title,
                description,
                sections_file,
                render_pdf,
                output,
            } => {
                let sections_json = std::fs::read_to_string(sections_file)
                    .map_err(|e| anyhow::anyhow!("Failed to read sections file: {}", e))?;
                let sections: Vec<quilt_cognitive::tree_rag::AssembledSection> =
                    serde_json::from_str(&sections_json)
                        .map_err(|e| anyhow::anyhow!("Invalid sections JSON: {}", e))?;

                let markdown = engine.assemble_document(title, description, &sections);

                if *render_pdf {
                    let pdf = engine.render_pdf(&markdown)?;
                    match output {
                        Some(path) => {
                            std::fs::write(path, &pdf)?;
                            println!("PDF written to: {}", path);
                        }
                        None => {
                            println!("PDF ({} bytes)", pdf.len());
                        }
                    }
                } else {
                    match output {
                        Some(path) => {
                            std::fs::write(path, &markdown)?;
                            println!("Markdown written to: {}", path);
                        }
                        None => {
                            println!("{}", markdown);
                        }
                    }
                }
            }
            TreeRagCommand::Status => {
                let status = engine.status().await?;
                println!("TreeRAG Status:");
                println!("  Total blocks: {}", status.total_blocks);
                println!("  Indexed: {}", status.indexed_blocks);
                println!("  Pending: {}", status.pending_blocks);
                if status.total_blocks > 0 {
                    let pct = (status.indexed_blocks as f64 / status.total_blocks as f64) * 100.0;
                    println!("  Coverage: {:.1}%", pct);
                }
            }
            TreeRagCommand::RebuildIndex { scope } => {
                let scope = match scope.as_ref() {
                    Some(s) => Some(self.parse_report_scope(s)?),
                    None => None,
                };
                let count = engine.rebuild_index(scope.as_ref()).await?;
                println!("Stale blocks needing summarization: {}", count);
            }
            TreeRagCommand::SaveSummary { block_id, summary } => {
                let bid = Uuid::parse_str(block_id)
                    .ok_or_else(|| anyhow::anyhow!("Invalid UUID: {}", block_id))?;
                engine.save_block_summary(bid, summary.clone()).await?;
                println!("Summary saved for block: {}", block_id);
            }
        }
        Ok(())
    }

    fn print_tree_node(&self, node: &quilt_cognitive::tree_rag::TreeNode, indent: usize) -> Result<()> {
        let prefix = "  ".repeat(indent);
        let summary = if node.summary.is_empty() {
            String::new()
        } else {
            format!(" - {}", node.summary.chars().take(50).collect::<String>())
        };
        println!("{}- {} [{}{}]", prefix, node.title, node.block_id, summary);
        for child in &node.children {
            self.print_tree_node(child, indent + 1)?;
        }
        Ok(())
    }

    fn parse_report_scope(&self, s: &str) -> Result<quilt_cognitive::tree_rag::ReportScope> {
        let s = s.to_lowercase();
        if s == "auto" {
            Ok(quilt_cognitive::tree_rag::ReportScope::Auto)
        } else if s == "all" {
            Ok(quilt_cognitive::tree_rag::ReportScope::AllPages)
        } else if let Some(days) = s.strip_prefix("journal:") {
            let n: u32 = days
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid journal days: {}", days))?;
            Ok(quilt_cognitive::tree_rag::ReportScope::JournalLast(n))
        } else if let Some(pages) = s.strip_prefix("pages:") {
            let page_names: Vec<String> = pages.split(',').map(|s| s.trim().to_string()).collect();
            Ok(quilt_cognitive::tree_rag::ReportScope::Pages(page_names))
        } else {
            Ok(quilt_cognitive::tree_rag::ReportScope::Auto)
        }
    }

    // ── Scheduler handlers ──────────────────────────────────────────────

    async fn run_scheduler(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        command: &SchedulerCommand,
    ) -> Result<()> {
        let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
        let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
        let summary_repo = Arc::new(SqliteBlockSummaryRepository::new(pool.clone()));
        let task_repo = Arc::new(SqliteScheduledTaskRepository::new(pool.clone()));

        let tree_rag = Arc::new(TreeRagEngine::new(
            block_repo,
            page_repo,
            summary_repo,
        ));
        let scheduler = Arc::new(TaskScheduler::new(task_repo.clone(), tree_rag));

        match command {
            SchedulerCommand::List => {
                let tasks = scheduler.list_tasks().await
                    .map_err(|e| anyhow::anyhow!(e))?;
                if tasks.is_empty() {
                    println!("No scheduled tasks.");
                } else {
                    println!("Scheduled tasks ({}):", tasks.len());
                    for task in &tasks {
                        let status = if task.enabled { "enabled" } else { "disabled" };
                        println!(
                            "  - {} ({}): {} — next: {}",
                            task.name,
                            format!("{:?}", task.task_type).to_lowercase(),
                            status,
                            task.next_run
                        );
                    }
                }
            }
            SchedulerCommand::Schedule {
                name,
                cron,
                task_type,
            } => {
                use quilt_domain::TaskType;
                let tt = match task_type.to_lowercase().as_str() {
                    "rebuildindex" => TaskType::RebuildIndex,
                    "cleanstalesummaries" => TaskType::CleanStaleSummaries,
                    "healthcheck" => TaskType::HealthCheck,
                    _ => anyhow::bail!(
                        "Unknown task type: {}. Valid: RebuildIndex, CleanStaleSummaries, HealthCheck",
                        task_type
                    ),
                };
                scheduler.schedule_task(name, cron, tt).await
                    .map_err(|e| anyhow::anyhow!(e))?;
                println!("Scheduled task '{}': {} ({})", name, cron, task_type);
            }
            SchedulerCommand::RunNow { name } => {
                scheduler.run_now(name).await
                    .map_err(|e| anyhow::anyhow!(e))?;
                println!("Task '{}' executed.", name);
            }
            SchedulerCommand::Delete { name } => {
                scheduler.delete_task(name).await
                    .map_err(|e| anyhow::anyhow!(e))?;
                println!("Task '{}' deleted.", name);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_init() {
        let cli = QuiltCLI::try_parse_from(["quilt", "init", "--name", "mygraph"]).unwrap();
        match cli.command {
            Command::Init { name } => assert_eq!(name, "mygraph"),
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_parse_page() {
        let cli = QuiltCLI::try_parse_from(["quilt", "page", "--name", "Test Page"]).unwrap();
        match cli.command {
            Command::Page { name } => assert_eq!(name, "Test Page"),
            _ => panic!("Expected Page command"),
        }
    }

    #[test]
    fn test_cli_parse_block() {
        let cli =
            QuiltCLI::try_parse_from(["quilt", "block", "--page", "Test", "--content", "Hello"])
                .unwrap();
        match cli.command {
            Command::Block {
                page,
                content,
                parent,
            } => {
                assert_eq!(page, "Test");
                assert_eq!(content, "Hello");
                assert!(parent.is_none());
            }
            _ => panic!("Expected Block command"),
        }
    }

    #[test]
    fn test_cli_parse_block_with_parent() {
        let cli = QuiltCLI::try_parse_from([
            "quilt",
            "block",
            "--page",
            "X",
            "--content",
            "Hi",
            "--parent",
            "uuid-here",
        ])
        .unwrap();
        match cli.command {
            Command::Block {
                page,
                content,
                parent,
            } => {
                assert_eq!(page, "X");
                assert_eq!(content, "Hi");
                assert_eq!(parent.as_deref(), Some("uuid-here"));
            }
            _ => panic!("Expected Block command"),
        }
    }

    #[test]
    fn test_cli_parse_journal() {
        let cli = QuiltCLI::try_parse_from(["quilt", "journal"]).unwrap();
        match cli.command {
            Command::Journal { date } => assert!(date.is_none()),
            _ => panic!("Expected Journal command"),
        }
    }

    #[test]
    fn test_cli_parse_journal_date() {
        let cli = QuiltCLI::try_parse_from(["quilt", "journal", "--date", "2024-01-15"]).unwrap();
        match cli.command {
            Command::Journal { date } => assert_eq!(date.as_deref(), Some("2024-01-15")),
            _ => panic!("Expected Journal command"),
        }
    }

    #[test]
    fn test_cli_parse_query() {
        let cli =
            QuiltCLI::try_parse_from(["quilt", "query", "--dsl", "(task todo)", "--limit", "50"])
                .unwrap();
        match cli.command {
            Command::Query { dsl, limit } => {
                assert_eq!(dsl, "(task todo)");
                assert_eq!(limit, 50);
            }
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_cli_parse_search() {
        let cli = QuiltCLI::try_parse_from(["quilt", "search", "--query", "rust"]).unwrap();
        match cli.command {
            Command::Search { query } => assert_eq!(query, "rust"),
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_cli_parse_serve() {
        let cli = QuiltCLI::try_parse_from(["quilt", "serve", "--port", "8080"]).unwrap();
        match cli.command {
            Command::Serve { port } => assert_eq!(port, 8080),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parse_list_pages() {
        let cli = QuiltCLI::try_parse_from(["quilt", "list-pages"]).unwrap();
        match cli.command {
            Command::ListPages => {}
            _ => panic!("Expected ListPages command"),
        }
    }

    #[test]
    fn test_cli_parse_page_info() {
        let cli = QuiltCLI::try_parse_from(["quilt", "page-info", "--name", "Test"]).unwrap();
        match cli.command {
            Command::PageInfo { name } => assert_eq!(name, "Test"),
            _ => panic!("Expected PageInfo command"),
        }
    }

    #[test]
    fn test_cli_parse_default_db_path() {
        let cli = QuiltCLI::try_parse_from(["quilt", "list-pages"]).unwrap();
        assert_eq!(cli.db_path.to_string_lossy(), "quilt.db");
    }

    #[test]
    fn test_cli_parse_verbose() {
        let cli = QuiltCLI::try_parse_from(["quilt", "--verbose", "list-pages"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parse_invalid_subcommand() {
        let result = QuiltCLI::try_parse_from(["quilt", "invalid-subcommand"]);
        assert!(result.is_err());
    }
}
