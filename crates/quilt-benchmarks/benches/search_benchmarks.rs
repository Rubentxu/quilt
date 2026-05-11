//! Search performance benchmarks
//!
//! Target: P95 < 100ms for FTS search

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlx::SqlitePool;
use uuid::Uuid;

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
            "order" REAL NOT NULL DEFAULT 0,
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
            tags TEXT NOT NULL DEFAULT '[]'
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
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create pages table");

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

/// Seeds test data with specified number of blocks.
async fn seed_test_data(pool: &SqlitePool, num_blocks: usize) {
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    // Insert test page
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("benchmark-page")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert test page");

    // Insert blocks with varied content for search testing
    let search_terms = [
        "rust programming language",
        "knowledge graph database",
        "local first software",
        "markdown editor",
        "task management system",
        "AI agent framework",
        "personal knowledge management",
        "bidirectional linking",
        "outliner application",
        "hierarchical note taking",
    ];

    for i in 0..num_blocks {
        let term = search_terms[i % search_terms.len()];
        let content = format!("Content {} about {}", i, term);

        sqlx::query(
            "INSERT INTO blocks (id, page_id, content, format, level, created_at, updated_at) VALUES (?, ?, ?, 'markdown', 1, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(page_id.to_string())
        .bind(content)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("failed to insert block");
    }

    // Insert into FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(pool)
        .await
        .expect("failed to populate FTS");
}

fn fts_search_benchmark(c: &mut Criterion) {
    let rt = tokio::Runtime::new().unwrap();

    let pool = rt.block_on(async { setup_test_db().await });

    let mut group = c.benchmark_group("fts_search");

    for size in [100, 1000, 10000].iter() {
        let pool_clone = pool.clone();
        rt.block_on(async { seed_test_data(&pool_clone, *size).await });

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.to_async(&rt).iter(|| async {
                let service = quilt_application::query_service::QueryService::new();
                let result = service
                    .prepare(black_box("(full-text-search \"rust\")"), 100)
                    .unwrap();
                let mut query = sqlx::query(&result.sql);
                for param in &result.params {
                    query = query.bind(param);
                }
                let rows = query.fetch_all(&pool).await.unwrap();
                rows.len()
            });
        });
    }

    group.finish();
}

fn search_snippet_benchmark(c: &mut Criterion) {
    let rt = tokio::Runtime::new().unwrap();

    let pool = rt.block_on(async { setup_test_db().await });
    rt.block_on(async { seed_test_data(&pool, 1000).await });

    c.bench_function("search_snippet_extraction", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate snippet extraction with FTS5 snippet function
            let result = sqlx::query_scalar::<_, String>(
                r#"
                SELECT snippet(blocks_fts, 0, '<mark>', '</mark>', '...', 64)
                FROM blocks_fts
                WHERE blocks_fts MATCH 'rust'
                LIMIT 10
                "#,
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            result.len()
        });
    });
}

criterion_group!(benches, fts_search_benchmark, search_snippet_benchmark);
criterion_main!(benches);
