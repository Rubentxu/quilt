//! Integration tests for ResourceUseCases — exercises graph snapshots,
//! page listing, journal listing, and tag listing with in-memory repos.
//!
//! Covers: graph_snapshot, list_pages, list_journals, list_tags.

use std::collections::HashMap;
use std::sync::Arc;

use quilt_application::use_cases::{ResourceUseCases, ResourceUseCasesImpl};
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::BlockFormat;
use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo, InMemoryTagRepo};

// ── Helpers ──────────────────────────────────────────────────

fn setup() -> ResourceUseCasesImpl<InMemoryBlockRepo, InMemoryPageRepo, InMemoryTagRepo> {
    let block_repo = InMemoryBlockRepo::new();
    let page_repo = InMemoryPageRepo::new();
    let tag_repo = InMemoryTagRepo::new();
    ResourceUseCasesImpl::new(block_repo, page_repo, tag_repo)
}

fn make_page(name: &str, journal: bool) -> Page {
    Page::new(PageCreate {
        name: name.to_string(),
        title: Some(format!("Title: {}", name)),
        namespace_id: None,
        journal_day: if journal {
            Some(quilt_domain::value_objects::JournalDay::from_ymd(2026, 6, 1).unwrap())
        } else {
            None
        },
        format: BlockFormat::Markdown,
        file_id: None,
        properties: std::collections::HashMap::new(),
    })
    .unwrap()
}

async fn insert_page(repo: &Arc<InMemoryPageRepo>, page: &Page) {
    repo.insert(page).await.unwrap();
}

// ── graph_snapshot ──────────────────────────────────────────

#[tokio::test]
async fn test_graph_snapshot_empty() {
    let use_cases = setup();

    let snapshot = use_cases.graph_snapshot().await.unwrap();
    assert_eq!(snapshot.pages_count, 0);
    assert_eq!(snapshot.journals_count, 0);
    assert_eq!(snapshot.blocks_count, 0);
    assert!(snapshot.recent_pages.is_empty());
}

#[tokio::test]
async fn test_graph_snapshot_with_pages() {
    let block_repo = InMemoryBlockRepo::new();
    let page_repo = InMemoryPageRepo::new();
    let tag_repo = InMemoryTagRepo::new();
    let use_cases = ResourceUseCasesImpl::new(block_repo.clone(), page_repo.clone(), tag_repo);

    let page1 = make_page("page-1", false);
    let page2 = make_page("journal-1", true);
    insert_page(&page_repo, &page1).await;
    insert_page(&page_repo, &page2).await;

    let snapshot = use_cases.graph_snapshot().await.unwrap();
    assert_eq!(snapshot.pages_count, 2);
    assert_eq!(snapshot.journals_count, 1);
    assert_eq!(snapshot.blocks_count, 0);
}

// ── list_pages ──────────────────────────────────────────────

#[tokio::test]
async fn test_list_pages_empty() {
    let use_cases = setup();
    let pages = use_cases.list_pages().await.unwrap();
    assert!(pages.is_empty());
}

#[tokio::test]
async fn test_list_pages_returns_all() {
    let block_repo = InMemoryBlockRepo::new();
    let page_repo = InMemoryPageRepo::new();
    let tag_repo = InMemoryTagRepo::new();
    let use_cases = ResourceUseCasesImpl::new(block_repo.clone(), page_repo.clone(), tag_repo);

    let page1 = make_page("alpha", false);
    let page2 = make_page("beta", false);
    insert_page(&page_repo, &page1).await;
    insert_page(&page_repo, &page2).await;

    let pages = use_cases.list_pages().await.unwrap();
    assert_eq!(pages.len(), 2);
    // Verify PageSummary fields
    let names: Vec<&str> = pages.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
}

#[tokio::test]
async fn test_list_pages_summary_has_is_journal_flag() {
    let block_repo = InMemoryBlockRepo::new();
    let page_repo = InMemoryPageRepo::new();
    let tag_repo = InMemoryTagRepo::new();
    let use_cases = ResourceUseCasesImpl::new(block_repo.clone(), page_repo.clone(), tag_repo);

    let journal_page = make_page("2026-06-01", true);
    insert_page(&page_repo, &journal_page).await;

    let pages = use_cases.list_pages().await.unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].is_journal);
}

// ── list_journals ───────────────────────────────────────────

#[tokio::test]
async fn test_list_journals_filters_non_journals() {
    let block_repo = InMemoryBlockRepo::new();
    let page_repo = InMemoryPageRepo::new();
    let tag_repo = InMemoryTagRepo::new();
    let use_cases = ResourceUseCasesImpl::new(block_repo.clone(), page_repo.clone(), tag_repo);

    let regular = make_page("regular", false);
    let journal = make_page("2026-06-01", true);
    insert_page(&page_repo, &regular).await;
    insert_page(&page_repo, &journal).await;

    let journals = use_cases.list_journals().await.unwrap();
    assert_eq!(journals.len(), 1);
    assert!(journals[0].journal_day.is_some());
}

// ── list_tags ───────────────────────────────────────────────

#[tokio::test]
async fn test_list_tags_empty() {
    let use_cases = setup();
    let tags = use_cases.list_tags().await.unwrap();
    assert!(tags.is_empty());
}

// ── PageSummary conversion ──────────────────────────────────

#[test]
fn test_page_summary_from_page() {
    use quilt_application::use_cases::PageSummary;

    let page = make_page("test-page", false);
    let summary = PageSummary::from(page);

    assert_eq!(summary.name, "test-page");
    assert_eq!(summary.title, Some("Title: test-page".to_string()));
    assert!(!summary.is_journal);
    assert!(!summary.id.is_empty());
}

#[test]
fn test_journal_summary_from_page() {
    use quilt_application::use_cases::JournalSummary;

    let page = make_page("2026-06-01", true);
    let summary = JournalSummary::from(page);

    assert_eq!(summary.name, "2026-06-01");
    assert!(summary.journal_day.is_some());
}
