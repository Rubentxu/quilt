//! Integration tests for MCP PageToolHandler.
//!
//! Uses mock PageUseCases to test: list_pages, get_page_blocks,
//! get_journal, missing params, and unknown tools.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use quilt_application::ApplicationError;
use quilt_application::use_cases::{PageUseCases, PageWithBlocks};
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::value_objects::BlockFormat;
use quilt_mcp::handlers::ToolHandler;
use quilt_mcp::handlers::page::PageToolHandler;
use serde_json::json;

// ── Mock PageUseCases ───────────────────────────────────────

struct MockPageUseCases {
    pages: Mutex<Vec<Page>>,
    error: Mutex<Option<String>>,
}

impl MockPageUseCases {
    fn new() -> Self {
        Self {
            pages: Mutex::new(Vec::new()),
            error: Mutex::new(None),
        }
    }

    fn seed_page(&self, name: &str, journal: bool) -> Page {
        let page = Page::new(PageCreate {
            name: name.to_string(),
            title: Some(format!("Title: {}", name)),
            namespace_id: None,
            journal_day: if journal {
                Some(quilt_domain::value_objects::JournalDay::from_ymd(2026, 6, 2).unwrap())
            } else {
                None
            },
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .unwrap();
        self.pages.lock().unwrap().push(page.clone());
        page
    }

    fn set_error(&self, msg: &str) {
        *self.error.lock().unwrap() = Some(msg.to_string());
    }
}

#[async_trait]
impl PageUseCases for MockPageUseCases {
    async fn create(&self, name: &str, title: Option<&str>) -> Result<Page, ApplicationError> {
        let page = Page::new(PageCreate {
            name: name.to_string(),
            title: title.map(|t| t.to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .map_err(|e| ApplicationError::Domain(e))?;
        self.pages.lock().unwrap().push(page.clone());
        Ok(page)
    }

    async fn update_properties(
        &self,
        _page_id: quilt_domain::value_objects::Uuid,
        _props: std::collections::HashMap<String, quilt_domain::value_objects::PropertyValue>,
    ) -> Result<Page, ApplicationError> {
        // Mock: this test fixture doesn't simulate property updates; tests
        // that exercise the real update_properties logic live in
        // quilt-infrastructure's SQLite test module.
        Err(ApplicationError::Validation(
            "MockPageUseCases::update_properties".into(),
        ))
    }

    async fn list(&self) -> Result<Vec<Page>, ApplicationError> {
        if let Some(err) = self.error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        Ok(self.pages.lock().unwrap().clone())
    }

    async fn get_blocks(&self, page_name: &str) -> Result<PageWithBlocks, ApplicationError> {
        if let Some(err) = self.error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        let page = self
            .pages
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.name == page_name)
            .cloned()
            .unwrap_or_else(|| {
                Page::new(PageCreate {
                    name: page_name.to_string(),
                    title: None,
                    namespace_id: None,
                    journal_day: None,
                    format: BlockFormat::Markdown,
                    file_id: None,
                    properties: std::collections::HashMap::new(),
                })
                .unwrap()
            });
        Ok(PageWithBlocks {
            page,
            blocks: vec![],
        })
    }

    async fn get_or_create_journal(&self, _date: &str) -> Result<Page, ApplicationError> {
        if let Some(err) = self.error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        let page = Page::new(PageCreate {
            name: "2026-06-02".to_string(),
            title: None,
            namespace_id: None,
            journal_day: Some(
                quilt_domain::value_objects::JournalDay::from_ymd(2026, 6, 2).unwrap(),
            ),
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .unwrap();
        Ok(page)
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Page>, ApplicationError> {
        if let Some(err) = self.error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        Ok(self.pages.lock().unwrap().iter().find(|p| p.name == name).cloned())
    }

    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Page>, ApplicationError> {
        if let Some(err) = self.error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        Ok(self.pages.lock().unwrap().clone())
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn handler() -> PageToolHandler {
    let mock = Arc::new(MockPageUseCases::new());
    PageToolHandler::new(mock)
}

// ── quilt_list_pages ────────────────────────────────────────

#[tokio::test]
async fn test_list_pages_empty() {
    let h = handler();
    let args = json!({});

    let result = h.execute("quilt_list_pages", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["count"], 0);
    assert!(parsed["pages"].is_array());
}

#[tokio::test]
async fn test_list_pages_with_data() {
    let mock = Arc::new(MockPageUseCases::new());
    mock.seed_page("alpha", false);
    mock.seed_page("beta", false);

    let h = PageToolHandler::new(mock);
    let result = h.execute("quilt_list_pages", &json!({})).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["count"], 2);
    let names: Vec<String> = parsed["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"alpha".to_string()));
    assert!(names.contains(&"beta".to_string()));
}

// ── quilt_get_page_blocks ───────────────────────────────────

#[tokio::test]
async fn test_get_page_blocks_success() {
    let mock = Arc::new(MockPageUseCases::new());
    mock.seed_page("my-page", false);
    let h = PageToolHandler::new(mock);

    let args = json!({ "page_name": "my-page" });
    let result = h.execute("quilt_get_page_blocks", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["page"]["name"], "my-page");
    assert!(parsed["blocks"].is_array());
    assert!(parsed["count"].is_number());
}

// T-01: page.updated_at must be present in the JSON output so that
// server-level evidence injection (Phase 3) can extract it for
// `page_updated_at` in the Evidence envelope.
#[tokio::test]
async fn test_get_page_blocks_includes_updated_at() {
    let mock = Arc::new(MockPageUseCases::new());
    mock.seed_page("my-page", false);
    let h = PageToolHandler::new(mock);

    let args = json!({ "page_name": "my-page" });
    let result = h.execute("quilt_get_page_blocks", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(
        parsed["page"]["updated_at"].is_string(),
        "page.updated_at must be present in quilt_get_page_blocks JSON (got: {:?})",
        parsed["page"]
    );
}

// T-01: page.updated_at must also be present in quilt_get_journal output.
#[tokio::test]
async fn test_get_journal_includes_updated_at() {
    let h = handler();
    let args = json!({ "date": "2026-06-02" });
    let result = h.execute("quilt_get_journal", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(
        parsed["page"]["updated_at"].is_string(),
        "page.updated_at must be present in quilt_get_journal JSON (got: {:?})",
        parsed["page"]
    );
}

#[tokio::test]
async fn test_get_page_blocks_missing_page_name() {
    let h = handler();
    let result = h.execute("quilt_get_page_blocks", &json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'page_name'"));
}

// ── quilt_get_journal ───────────────────────────────────────

#[tokio::test]
async fn test_get_journal_success() {
    let h = handler();
    let args = json!({ "date": "2026-06-02" });

    let result = h.execute("quilt_get_journal", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["page"]["journal_day"].is_number());
    assert!(parsed["blocks"].is_array());
}

#[tokio::test]
async fn test_get_journal_missing_date() {
    let h = handler();
    let result = h.execute("quilt_get_journal", &json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'date'"));
}

// ── Unknown tool ────────────────────────────────────────────

#[tokio::test]
async fn test_unknown_tool() {
    let h = handler();
    let result = h.execute("nonexistent", &json!({})).await;
    assert!(result.is_err());
}

// ── Tool listing ────────────────────────────────────────────

#[test]
fn test_tools_list() {
    let h = handler();
    let tools = h.tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"quilt_list_pages"));
    assert!(names.contains(&"quilt_get_page_blocks"));
    assert!(names.contains(&"quilt_get_journal"));
}
