//! Integration tests for PageCommand and PageQuery handlers
//!
//! These tests verify the full stack from command/query handlers through to the database.

use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::repositories::PageRepository;
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Sets up an in-memory SQLite database with the full schema.
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to connect to memory DB");

    // Create blocks table
    sqlx::query(
        r#"
        CREATE TABLE blocks (
            id TEXT PRIMARY KEY NOT NULL,
            page_id TEXT NOT NULL,
            parent_id TEXT,
            order_index REAL NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 1,
            format TEXT NOT NULL DEFAULT 'markdown',
            marker TEXT,
            priority TEXT,
            content TEXT NOT NULL DEFAULT '',
            properties TEXT NOT NULL DEFAULT '{}',
            scheduled INTEGER,
            deadline INTEGER,
            start_time INTEGER,
            repeated INTEGER,
            logbook INTEGER,
            collapsed INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            refs TEXT NOT NULL DEFAULT '[]',
            tags TEXT NOT NULL DEFAULT '[]',
            deleted_at INTEGER,
            journal_day INTEGER,
            updated_journal_day INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create blocks table");

    // Create pages table
    sqlx::query(
        r#"
        CREATE TABLE pages (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            title TEXT,
            namespace_id TEXT,
            journal_day INTEGER,
            format TEXT NOT NULL DEFAULT 'markdown',
            file_id TEXT,
            original_name TEXT,
            journal INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create pages table");

    // Create tags table
    sqlx::query(
        r#"
        CREATE TABLE tags (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create tags table");

    // Create block_tags table
    sqlx::query(
        r#"
        CREATE TABLE block_tags (
            block_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            PRIMARY KEY (block_id, tag_id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create block_tags table");

    // Create refs table
    sqlx::query(
        r#"
        CREATE TABLE refs (
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            PRIMARY KEY (source_id, target_id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create refs table");

    // Create FTS virtual table
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE blocks_fts USING fts5(
            content,
            content=blocks,
            content_rowid=rowid
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create FTS table");

    pool
}

/// Seed test data and return the page_id
async fn seed_test_data(pool: &SqlitePool) -> Uuid {
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    // Insert test page
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("test-page")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert test page");

    page_id
}

// =============================================================================
// PageCommand Handler Tests
// =============================================================================

#[tokio::test]
async fn test_page_command_create_page() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));

    // Create a page
    let result = command
        .create("my-test-page".to_string(), Some("My Test Page".to_string()), None)
        .await;

    assert!(result.is_ok());
    let page = result.unwrap();
    assert_eq!(page.name, "my-test-page");
    assert_eq!(page.title, Some("My Test Page".to_string()));
    assert!(!page.journal);
}

#[tokio::test]
async fn test_page_command_create_with_namespace() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));

    let namespace_id = Uuid::new_v4();

    // Create a page with namespace
    let result = command
        .create(
            "namespaced-page".to_string(),
            Some("Namespaced Page".to_string()),
            Some(namespace_id),
        )
        .await;

    assert!(result.is_ok());
    let page = result.unwrap();
    assert_eq!(page.namespace_id, Some(namespace_id));
}

#[tokio::test]
async fn test_page_command_create_journal() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));

    let day = JournalDay::from_i32(20260521).unwrap();

    // Create a journal page
    let result = command.create_journal(day).await;

    assert!(result.is_ok());
    let page = result.unwrap();
    assert!(page.journal);
    assert_eq!(page.journal_day, Some(day));
}

#[tokio::test]
async fn test_page_command_rename_page() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));

    // Create a page
    let page = command
        .create("old-name".to_string(), None, None)
        .await
        .unwrap();

    // Rename the page
    let result = command.rename(page.id, "new-name").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "new-name");
}

#[tokio::test]
async fn test_page_command_delete_page() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));

    // Create a page
    let page = command
        .create("page-to-delete".to_string(), None, None)
        .await
        .unwrap();

    // Delete the page
    let result = command.delete(page.id).await;
    assert!(result.is_ok());

    // Verify it's gone
    let retrieved = repo.get_by_name("page-to-delete").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_page_command_hard_delete_page() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));

    // Create a page
    let page = command
        .create("page-to-hard-delete".to_string(), None, None)
        .await
        .unwrap();

    // Hard delete the page
    let result = command.hard_delete(page.id).await;
    assert!(result.is_ok());

    // Verify it's completely gone
    let retrieved = repo.get_by_name("page-to-hard-delete").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_page_command_restore_page() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));

    // Create a page
    let page = command
        .create("page-to-restore".to_string(), None, None)
        .await
        .unwrap();

    // Delete the page
    command.delete(page.id).await.unwrap();

    // Restore the page
    let result = command.restore(page.id).await;
    assert!(result.is_ok());

    // Verify it's back
    let retrieved = repo.get_by_name("page-to-restore").await.unwrap();
    assert!(retrieved.is_some());
}

// =============================================================================
// PageQuery Handler Tests
// =============================================================================

#[tokio::test]
async fn test_page_query_get_page() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqlitePageRepository::new(pool.clone());
    let query = quilt_application::queries::PageQuery::new(Arc::new(repo));

    // Query the page
    let result = query.get(page_id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[tokio::test]
async fn test_page_query_get_by_name() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let repo = SqlitePageRepository::new(pool.clone());
    let query = quilt_application::queries::PageQuery::new(Arc::new(repo));

    // Query by name
    let result = query.get_by_name("test-page").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[tokio::test]
async fn test_page_query_get_all() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));
    let query = quilt_application::queries::PageQuery::new(Arc::new(repo));

    // Create multiple pages
    command
        .create("page-1".to_string(), None, None)
        .await
        .unwrap();
    command
        .create("page-2".to_string(), None, None)
        .await
        .unwrap();

    // Query all
    let result = query.get_all().await;
    assert!(result.is_ok());
    assert!(result.unwrap().len() >= 2);
}

#[tokio::test]
async fn test_page_query_get_recent() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));
    let query = quilt_application::queries::PageQuery::new(Arc::new(repo));

    // Create pages
    for i in 0..5 {
        command
            .create(format!("recent-page-{}", i), None, None)
            .await
            .unwrap();
    }

    // Query recent
    let result = query.get_recent(3).await;
    assert!(result.is_ok());
    assert!(result.unwrap().len() <= 3);
}

#[tokio::test]
async fn test_page_query_get_journal() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo));
    let query = quilt_application::queries::PageQuery::new(Arc::new(repo));

    let day = JournalDay::from_i32(20260521).unwrap();

    // Create a journal page
    command.create_journal(day).await.unwrap();

    // Query journal
    let result = query.get_journal(day).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

// =============================================================================
// Page Handler List Tests
// =============================================================================

#[tokio::test]
async fn test_page_handler_list_pages() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());

    // Create some pages
    let page1 = Page::new(PageCreate {
        name: "list-test-page-1".to_string(),
        title: Some("List Test Page 1".to_string()),
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .expect("Page creation should succeed");

    let page2 = Page::new(PageCreate {
        name: "list-test-page-2".to_string(),
        title: Some("List Test Page 2".to_string()),
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .expect("Page creation should succeed");

    repo.insert(&page1).await.expect("Insert should succeed");
    repo.insert(&page2).await.expect("Insert should succeed");

    // List pages using the query
    let pages = repo.get_all().await.expect("List should succeed");

    assert!(!pages.is_empty());

    // Verify our pages exist in the list
    let names: Vec<_> = pages.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"list-test-page-1"));
    assert!(names.contains(&"list-test-page-2"));
}

// =============================================================================
// Page Handler E2E Flow Tests
// =============================================================================

#[tokio::test]
async fn test_page_handler_full_flow() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo.clone()));
    let query = quilt_application::queries::PageQuery::new(Arc::new(repo));

    // 1. Create a page
    let page = command
        .create("flow-test-page".to_string(), Some("Flow Test Page".to_string()), None)
        .await
        .unwrap();

    // 2. Verify page exists
    let retrieved = query.get_by_name("flow-test-page").await.unwrap().unwrap();
    assert_eq!(retrieved.name, "flow-test-page");

    // 3. Rename the page
    let renamed = command.rename(page.id, "flow-test-page-renamed").await.unwrap();
    assert_eq!(renamed.name, "flow-test-page-renamed");

    // 4. Delete the page
    command.delete(page.id).await.unwrap();

    // 5. Verify page is gone
    let result = query.get_by_name("flow-test-page-renamed").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_journal_page_lifecycle() {
    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());
    let command = quilt_application::commands::PageCommand::new(Arc::new(repo.clone()));
    let query = quilt_application::queries::PageQuery::new(Arc::new(repo));

    let day = JournalDay::from_i32(20260521).unwrap();

    // 1. Create journal page
    let journal = command.create_journal(day).await.unwrap();
    assert!(journal.journal);
    assert_eq!(journal.journal_day, Some(day));

    // 2. Verify journal exists
    let retrieved = query.get_journal(day).await.unwrap().unwrap();
    assert_eq!(retrieved.journal_day, Some(day));

    // 3. Delete journal page
    command.delete(journal.id).await.unwrap();

    // 4. Verify journal is gone
    let result = query.get_journal(day).await.unwrap();
    assert!(result.is_none());
}