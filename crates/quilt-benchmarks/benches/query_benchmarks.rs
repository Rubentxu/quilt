//! Query performance benchmarks
//!
//! Target: P95 < 100ms

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

    // Insert blocks with various markers and priorities
    let markers = ["todo", "done", "later", "now", "cancelled"];
    let priorities = ["a", "b", "c"];

    for i in 0..num_blocks {
        let marker = markers[i % markers.len()];
        let priority = if marker == "todo" {
            priorities[i % priorities.len()]
        } else {
            ""
        };
        let content = format!("Benchmark task {} with {} priority", i, marker);

        sqlx::query(
            "INSERT INTO blocks (id, page_id, content, marker, priority, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'markdown', 1, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(page_id.to_string())
        .bind(content)
        .bind(marker)
        .bind(priority)
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

fn query_parser_benchmark(c: &mut Criterion) {
    c.bench_function("query_parser_simple", |b| {
        b.iter(|| {
            let parser = quilt_query::QueryParser;
            let _ = parser.parse(black_box("(task todo)"));
        });
    });

    c.bench_function("query_parser_complex", |b| {
        b.iter(|| {
            let parser = quilt_query::QueryParser;
            let _ = parser.parse(black_box(
                "(and (or (not (task todo)) (task done)) (priority a))",
            ));
        });
    });

    c.bench_function("query_parser_page_ref", |b| {
        b.iter(|| {
            let parser = quilt_query::QueryParser;
            let _ = parser.parse(black_box("[[My Page]]"));
        });
    });
}

fn query_execution_benchmark(c: &mut Criterion) {
    let rt = tokio::Runtime::new().unwrap();
    let pool = rt.block_on(async { setup_test_db().await });
    rt.block_on(async { seed_test_data(&pool, 100).await });

    let mut group = c.benchmark_group("query_execution");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.to_async(&rt).iter(|| async {
                let service = quilt_application::query_service::QueryService::new();
                let result = service.prepare(black_box("(task todo)"), 100).unwrap();
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

criterion_group!(benches, query_parser_benchmark, query_execution_benchmark);
criterion_main!(benches);
