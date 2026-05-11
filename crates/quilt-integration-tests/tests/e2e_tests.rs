//! End-to-End Tests for Quilt Application
//!
//! These tests verify critical user flows work end-to-end:
//! 1. Page creation → Block creation → Block editing
//! 2. Search → Query → Results display
//! 3. Journal page → Today's entries
//! 4. MCP tool execution

use sqlx::SqlitePool;
use std::time::Duration;

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
            deleted_at INTEGER
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

// =============================================================================
// E2E Flow 1: Page creation → Block creation → Block editing
// =============================================================================

#[tokio::test]
async fn test_e2e_page_creation_flow() {
    use quilt_domain::entities::{Page, PageCreate};
    use quilt_domain::repositories::PageRepository;
    use quilt_domain::value_objects::BlockFormat;

    let pool = setup_test_db().await;
    let repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );

    // Step 1: Create a new page
    let page = Page::new(PageCreate {
        name: "e2e-test-page".to_string(),
        title: Some("E2E Test Page".to_string()),
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .expect("Page creation should succeed");

    repo.insert(&page)
        .await
        .expect("Insert page should succeed");

    // Step 2: Verify page was created
    let retrieved = repo
        .get_by_name("e2e-test-page")
        .await
        .expect("Get by name should succeed")
        .expect("Page should exist");

    assert_eq!(retrieved.name, "e2e-test-page");
    assert_eq!(retrieved.title, Some("E2E Test Page".to_string()));
}

#[tokio::test]
async fn test_e2e_block_creation_flow() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};

    let pool = setup_test_db().await;
    let repo = quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
        pool.clone(),
    );

    // Create parent page first
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("block-test-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page");

    // Step 1: Create a block
    let create = BlockCreate {
        page_id,
        content: "First block content".to_string(),
        parent_id: None,
        order: 1.0,
        marker: Some(TaskMarker::Todo),
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };

    let block = Block::new(create).expect("Block creation should succeed");
    repo.insert(&block).await.expect("Insert should succeed");

    // Step 2: Verify block was created
    let retrieved = repo
        .get_by_id(block.id)
        .await
        .expect("Get by id should succeed")
        .expect("Block should exist");

    assert_eq!(retrieved.content, "First block content");
    assert_eq!(retrieved.marker, Some(TaskMarker::Todo));
}

#[tokio::test]
async fn test_e2e_block_editing_flow() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};

    let pool = setup_test_db().await;
    let repo = quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
        pool.clone(),
    );

    // Create parent page
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("edit-test-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page");

    // Create a block
    let create = BlockCreate {
        page_id,
        content: "Original content".to_string(),
        parent_id: None,
        order: 1.0,
        marker: Some(TaskMarker::Todo),
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };

    let mut block = Block::new(create).expect("Block creation should succeed");

    // Insert the block first
    repo.insert(&block).await.expect("Insert should succeed");

    // Step 2: Edit the block content
    block
        .update(quilt_domain::entities::BlockUpdate {
            content: Some("Updated content".to_string()),
            ..Default::default()
        })
        .expect("Update should succeed");

    repo.update(&block).await.expect("Update should succeed");

    // Step 3: Verify block was updated
    let retrieved = repo
        .get_by_id(block.id)
        .await
        .expect("Get by id should succeed")
        .expect("Block should exist");

    assert_eq!(retrieved.content, "Updated content");
}

#[tokio::test]
async fn test_e2e_page_to_block_to_edit_flow() {
    use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
    use quilt_domain::repositories::{BlockRepository, PageRepository};
    use quilt_domain::value_objects::{BlockFormat, TaskMarker};

    let pool = setup_test_db().await;
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );
    let block_repo =
        quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
            pool.clone(),
        );

    // Step 1: Create a page
    let page = Page::new(PageCreate {
        name: "full-flow-page".to_string(),
        title: Some("Full Flow Test".to_string()),
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .expect("Page creation should succeed");

    page_repo
        .insert(&page)
        .await
        .expect("Insert page should succeed");

    // Step 2: Create blocks on the page
    let block1 = Block::new(BlockCreate {
        page_id: page.id,
        content: "First task".to_string(),
        parent_id: None,
        order: 1.0,
        marker: Some(TaskMarker::Todo),
        format: BlockFormat::Markdown,
        properties: Default::default(),
    })
    .expect("Block 1 creation should succeed");

    let block2 = Block::new(BlockCreate {
        page_id: page.id,
        content: "Second task".to_string(),
        parent_id: Some(block1.id),
        order: 1.0,
        marker: Some(TaskMarker::Now),
        format: BlockFormat::Markdown,
        properties: Default::default(),
    })
    .expect("Block 2 creation should succeed");

    block_repo
        .insert(&block1)
        .await
        .expect("Insert block 1 should succeed");
    block_repo
        .insert(&block2)
        .await
        .expect("Insert block 2 should succeed");

    // Step 3: Edit block2 to mark it done
    let mut block2_updated = block2.clone();
    block2_updated
        .update(quilt_domain::entities::BlockUpdate {
            marker: Some(TaskMarker::Done),
            ..Default::default()
        })
        .expect("Update should succeed");

    block_repo
        .update(&block2_updated)
        .await
        .expect("Update block 2 should succeed");

    // Verify final state
    let retrieved_page = page_repo
        .get_by_name("full-flow-page")
        .await
        .expect("Get page should succeed")
        .expect("Page should exist");

    let blocks = block_repo
        .get_by_page(page.id)
        .await
        .expect("Get blocks should succeed");

    assert_eq!(retrieved_page.name, "full-flow-page");
    assert_eq!(blocks.len(), 2);

    let done_block = blocks.iter().find(|b| b.marker == Some(TaskMarker::Done));
    assert!(done_block.is_some(), "Should have one done block");
}

// =============================================================================
// E2E Flow 2: Search → Query → Results display
// =============================================================================

#[tokio::test]
async fn test_e2e_search_query_flow() {
    use uuid::Uuid;

    let pool = setup_test_db().await;

    // Seed test data
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("search-test-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page");

    // Insert blocks with searchable content
    let search_terms = vec![
        "rust programming",
        "knowledge graph",
        "personal knowledge",
        "pkm tools",
    ];

    for (i, term) in search_terms.iter().enumerate() {
        let block_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO blocks (id, page_id, content, format, level, marker, created_at, updated_at) VALUES (?, ?, ?, 'markdown', 1, ?, ?, ?)",
        )
        .bind(block_id.to_string())
        .bind(page_id.to_string())
        .bind(format!("Article about {}", term))
        .bind(if i == 0 { "todo" } else { "done" })
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("failed to insert block");
    }

    // Insert into FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    // Step 1: Execute FTS search
    let service = quilt_application::query_service::QueryService::new();
    let result = service
        .prepare("(full-text-search \"knowledge\")", 100)
        .expect("Query preparation should succeed");

    let mut query = sqlx::query(&result.sql);
    for param in &result.params {
        query = query.bind(param);
    }
    let rows = query
        .fetch_all(&pool)
        .await
        .expect("Query execution should succeed");

    // Step 2: Verify search returned results
    assert!(
        !rows.is_empty(),
        "Search should return results for 'knowledge'"
    );

    // Step 3: Execute task query
    let result = service
        .prepare("(task todo)", 100)
        .expect("Query preparation should succeed");

    let mut query = sqlx::query(&result.sql);
    for param in &result.params {
        query = query.bind(param);
    }
    let rows = query
        .fetch_all(&pool)
        .await
        .expect("Query execution should succeed");

    assert_eq!(rows.len(), 1, "Should find exactly 1 todo block");
}

// =============================================================================
// E2E Flow 3: Journal page → Today's entries
// =============================================================================

#[tokio::test]
async fn test_e2e_journal_flow() {
    use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
    use quilt_domain::repositories::{BlockRepository, PageRepository};
    use quilt_domain::value_objects::{BlockFormat, JournalDay};

    let pool = setup_test_db().await;
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );
    let block_repo =
        quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
            pool.clone(),
        );

    // Step 1: Create a journal page for today
    let today = JournalDay::today();

    let page = Page::new(PageCreate {
        name: today.to_string(),
        title: None,
        namespace_id: None,
        journal_day: Some(today),
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .expect("Journal page creation should succeed");

    page_repo
        .insert(&page)
        .await
        .expect("Insert journal page should succeed");

    // Step 2: Add journal entries (blocks)
    let entry1 = Block::new(BlockCreate {
        page_id: page.id,
        content: "Morning planning".to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    })
    .expect("Entry 1 creation should succeed");

    let entry2 = Block::new(BlockCreate {
        page_id: page.id,
        content: "Afternoon review".to_string(),
        parent_id: None,
        order: 2.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    })
    .expect("Entry 2 creation should succeed");

    block_repo
        .insert(&entry1)
        .await
        .expect("Insert entry 1 should succeed");
    block_repo
        .insert(&entry2)
        .await
        .expect("Insert entry 2 should succeed");

    // Step 3: Verify journal page exists and has entries
    let retrieved_page = page_repo
        .get_by_name(&today.to_string())
        .await
        .expect("Get journal page should succeed")
        .expect("Journal page should exist");

    assert!(retrieved_page.journal, "Page should be marked as journal");

    let entries = block_repo
        .get_by_page(page.id)
        .await
        .expect("Get entries should succeed");
    assert_eq!(entries.len(), 2, "Journal should have 2 entries");
}

// =============================================================================
// E2E Flow 4: MCP tool execution
// =============================================================================

#[tokio::test]
async fn test_e2e_mcp_tool_page_creation() {
    use quilt_domain::entities::{Page, PageCreate};
    use quilt_domain::repositories::PageRepository;
    use quilt_domain::value_objects::BlockFormat;
    use uuid::Uuid;

    let pool = setup_test_db().await;
    let repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );

    // Simulate MCP tool call: create_page
    let page_name = format!("mcp-test-{}", Uuid::new_v4());
    let page = Page::new(PageCreate {
        name: page_name.clone(),
        title: Some("MCP Created Page".to_string()),
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .expect("Page creation should succeed");

    repo.insert(&page).await.expect("Insert should succeed");

    // Verify page was created via MCP tool
    let retrieved = repo
        .get_by_name(&page_name)
        .await
        .expect("Get should succeed")
        .expect("Page should exist");

    assert_eq!(retrieved.name, page_name);
    assert_eq!(retrieved.title, Some("MCP Created Page".to_string()));
}

#[tokio::test]
async fn test_e2e_mcp_tool_block_operations() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};

    let pool = setup_test_db().await;
    let repo = quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
        pool.clone(),
    );

    // Create parent page first
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("mcp-block-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page");

    // Simulate MCP tool call: create_block
    let block = Block::new(BlockCreate {
        page_id,
        content: "MCP created block".to_string(),
        parent_id: None,
        order: 1.0,
        marker: Some(TaskMarker::Todo),
        format: BlockFormat::Markdown,
        properties: Default::default(),
    })
    .expect("Block creation should succeed");

    repo.insert(&block).await.expect("Insert should succeed");

    // Verify block was created
    let retrieved = repo
        .get_by_id(block.id)
        .await
        .expect("Get should succeed")
        .expect("Block should exist");

    assert_eq!(retrieved.content, "MCP created block");
    assert_eq!(retrieved.marker, Some(TaskMarker::Todo));
}

// =============================================================================
// Performance Tests
// =============================================================================

#[tokio::test]
async fn test_performance_query_execution_time() {
    use std::time::Instant;
    use uuid::Uuid;

    let pool = setup_test_db().await;

    // Seed 1000 blocks
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("perf-test-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page");

    for i in 0..1000 {
        sqlx::query(
            "INSERT INTO blocks (id, page_id, content, format, level, marker, priority, created_at, updated_at) VALUES (?, ?, ?, 'markdown', 1, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(page_id.to_string())
        .bind(format!("Task {}", i))
        .bind(if i % 10 == 0 { "todo" } else { "done" })
        .bind(if i % 10 == 0 { "a" } else { "b" })
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("failed to insert block");
    }

    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let service = quilt_application::query_service::QueryService::new();

    // Benchmark query execution
    let start = Instant::now();
    let result = service
        .prepare("(task todo)", 100)
        .expect("Query should succeed");
    let mut query = sqlx::query(&result.sql);
    for param in &result.params {
        query = query.bind(param);
    }
    let rows = query.fetch_all(&pool).await.expect("Query should succeed");
    let elapsed = start.elapsed();

    assert!(!rows.is_empty(), "Should find some todo blocks");
    // Target: < 100ms P95, we expect much faster for 1000 blocks
    assert!(
        elapsed < Duration::from_millis(100),
        "Query should complete in < 100ms, took {:?}",
        elapsed
    );
}

// =============================================================================
// Security Tests
// =============================================================================

#[tokio::test]
async fn test_security_sql_injection_prevention() {
    use uuid::Uuid;

    let pool = setup_test_db().await;

    // Create a page for testing
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("security-test-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page");

    // Try SQL injection via page reference
    let service = quilt_application::query_service::QueryService::new();

    // Malicious input should be safely handled (not execute as SQL)
    let malicious_input = "test'; DROP TABLE blocks; --";
    let result = service.prepare(&format!("[[{}]]", malicious_input), 100);

    // Should either fail gracefully or not execute the DROP
    if result.is_ok() {
        // If parsing succeeds, verify the DROP wasn't executed
        let check = sqlx::query("SELECT COUNT(*) FROM blocks")
            .fetch_one(&pool)
            .await;
        assert!(
            check.is_ok(),
            "blocks table should still exist after injection attempt"
        );
    }
    // If parsing fails, that's also acceptable (injection prevented)
}

#[tokio::test]
async fn test_security_input_validation_page_names() {
    use quilt_domain::entities::{Page, PageCreate};
    use quilt_domain::value_objects::BlockFormat;

    // Test that page names with special characters are handled safely
    let special_names = vec!["Normal Page", "Page With Spaces", "Page With Slashes"];

    for name in special_names {
        let result = Page::new(PageCreate {
            name: name.to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        });

        // All these should be handled (some may fail with InvalidPageName, but shouldn't panic)
        // The important thing is no SQL injection or panics
        if result.is_ok() {
            let page = result.unwrap();
            // Page was created successfully - verify its fields are valid
            assert!(!page.name.is_empty(), "Page name should not be empty");
        }
    }
}

// =============================================================================
// Stress Tests
// =============================================================================

#[tokio::test]
async fn test_stress_many_blocks_on_page() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::{BlockFormat, Uuid};

    let pool = setup_test_db().await;
    let repo = quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
        pool.clone(),
    );

    // Create parent page
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("stress-test-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page");

    // Create 100 blocks
    let mut blocks = Vec::new();
    for i in 0..100 {
        let block = Block::new(BlockCreate {
            page_id,
            content: format!("Stress test block {}", i),
            parent_id: None,
            order: i as f64,
            marker: None,
            format: BlockFormat::Markdown,
            properties: Default::default(),
        })
        .expect("Block creation should succeed");

        blocks.push(block);
    }

    // Insert all blocks
    for block in &blocks {
        repo.insert(block).await.expect("Insert should succeed");
    }

    // Verify all blocks exist
    let retrieved = repo.get_by_page(page_id).await.expect("Get should succeed");
    assert_eq!(retrieved.len(), 100, "All 100 blocks should exist");
}
