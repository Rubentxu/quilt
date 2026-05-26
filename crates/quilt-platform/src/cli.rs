//! CLI implementation for Quilt

use anyhow::Result;
use clap::{Parser, Subcommand};
use quilt_application::bootstrap::AppServices;
use quilt_application::use_cases::*;
use quilt_application::{JournalDay, Uuid};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

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
}

impl QuiltCLI {
    pub async fn run(&self, services: &AppServices) -> Result<()> {
        match &self.command {
            Command::Init { name } => self.run_init(name)?,
            Command::Open { name } => self.run_open(&*services.resource, name.as_deref()).await?,
            Command::Page { name } => self.run_page(&*services.page, name).await?,
            Command::Block {
                page,
                content,
                parent,
            } => {
                self.run_block(&*services.block, page, content, parent.as_deref())
                    .await?
            }
            Command::Journal { date } => self.run_journal(&*services.page, date.as_deref()).await?,
            Command::Query { dsl, limit } => self.run_query(&*services.search, dsl, *limit).await?,
            Command::Search { query } => self.run_search(&*services.search, query).await?,
            Command::Serve { port } => {
                println!("MCP server on port {} — run via dedicated binary", port);
            }
            Command::ListPages => self.run_list_pages(&*services.page).await?,
            Command::PageInfo { name } => self.run_page_info(&*services.page, name).await?,
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

    async fn run_page(&self, page_uc: &dyn PageUseCases, name: &str) -> Result<()> {
        page_uc.create(name, None).await?;
        println!("✓ Page created: {}", name);
        Ok(())
    }

    async fn run_block(
        &self,
        block_uc: &dyn BlockUseCases,
        page_name: &str,
        content: &str,
        parent_id: Option<&str>,
    ) -> Result<()> {
        let parent_uuid = parent_id
            .map(|s| Uuid::parse_str(s).ok_or_else(|| anyhow::anyhow!("Invalid UUID: {}", s)))
            .transpose()?;

        block_uc
            .create_with_page(page_name, content, parent_uuid, None, HashMap::new())
            .await?;
        println!("✓ Block created on page '{}': {}", page_name, content);
        Ok(())
    }

    async fn run_journal(&self, page_uc: &dyn PageUseCases, date_str: Option<&str>) -> Result<()> {
        let day = match date_str {
            Some(s) => JournalDay::from_str(s)?,
            None => JournalDay::from_datetime(&chrono::Utc::now()),
        };

        let page = page_uc.get_or_create_journal(&day.to_string()).await?;
        println!("✓ Journal created: {} (day {})", page.name, day.as_i32());
        Ok(())
    }

    async fn run_query(
        &self,
        search_uc: &dyn SearchUseCases,
        dsl: &str,
        limit: usize,
    ) -> Result<()> {
        match search_uc.query(dsl, limit).await {
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

    async fn run_search(&self, search_uc: &dyn SearchUseCases, query: &str) -> Result<()> {
        println!("Search: {}", query);
        let results = search_uc
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

    async fn run_list_pages(&self, page_uc: &dyn PageUseCases) -> Result<()> {
        let pages = page_uc.list().await?;
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

    async fn run_page_info(&self, page_uc: &dyn PageUseCases, name: &str) -> Result<()> {
        let page_with_blocks = page_uc.get_blocks(name).await?;

        let page = &page_with_blocks.page;
        let blocks = &page_with_blocks.blocks;

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
        println!("  Blocks: {}", blocks.len());
        println!("  Created: {}", page.created_at);
        println!("  Updated: {}", page.updated_at);

        // Show recent blocks
        if !blocks.is_empty() {
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

    async fn run_open(&self, resource_uc: &dyn ResourceUseCases, name: Option<&str>) -> Result<()> {
        let snapshot = resource_uc.graph_snapshot().await?;
        println!(
            "Graph: {} (database: {})",
            name.unwrap_or("quilt"),
            self.db_path.display()
        );
        println!("  Pages: {}", snapshot.pages_count);
        println!("  Blocks: {}", snapshot.blocks_count);
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
