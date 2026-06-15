//! migrate-comments — one-shot migration from block-as-comment to annotations table.
//!
//! Reads every block whose `properties` JSON contains `type: "comment"` and
//! inserts a corresponding row into the `annotations` table.
//!
//! Two-pass migration:
//!   Pass 1: Insert all comments as annotations (block_id = comment's parent_id).
//!   Pass 2: For replies (comments whose parent is also a comment), fix
//!           block_id and set parent_annotation_id.
//!
//! Usage:
//!   cargo run --bin migrate-comments -- --database-url /path/to/quilt.db --dry-run
//!   cargo run --bin migrate-comments -- --database-url /path/to/quilt.db

use anyhow::{Context, Result};
use clap::Parser;
use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(
    name = "migrate-comments",
    about = "Migrate block-as-comment data to annotations"
)]
struct Args {
    #[arg(short = 'd', long, default_value = "quilt.db")]
    database_url: String,

    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!(db = %args.database_url, dry = args.dry_run, "Starting annotation migration");

    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", args.database_url))
        .await
        .context("Failed to connect to SQLite database")?;

    // Verify annotations table exists
    sqlx::query("SELECT 1 FROM annotations LIMIT 0")
        .execute(&pool)
        .await
        .context("annotations table not found — run migrations first")?;

    let rows = sqlx::query(
        r#"
        SELECT id, parent_id, content, properties, created_at
        FROM blocks
        WHERE json_extract(properties, '$.type') = 'comment'
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    .context("Failed to query comment blocks")?;

    info!(count = rows.len(), "Found comment blocks");

    if rows.is_empty() {
        info!("No comment blocks — nothing to migrate");
        return Ok(());
    }

    let mut migrated = 0u64;
    let mut skipped_existing = 0u64;
    let mut skipped_no_parent = 0u64;

    // ── Pass 1: Insert all root annotations ──
    for row in &rows {
        let comment_id: Vec<u8> = row.get("id");
        let parent_id: Option<Vec<u8>> = row.get("parent_id");

        let exists = sqlx::query("SELECT COUNT(*) FROM annotations WHERE id = ?")
            .bind(&comment_id)
            .fetch_one(&pool)
            .await
            .map(|r| r.get::<i64, _>(0) > 0)
            .unwrap_or(false);

        if exists {
            skipped_existing += 1;
            continue;
        }

        let block_id = match &parent_id {
            Some(pid) => pid.clone(),
            None => {
                skipped_no_parent += 1;
                warn!("Comment has no parent_id, skipping");
                continue;
            }
        };

        let content: String = row.get("content");
        let properties_raw: Vec<u8> = row.get::<Vec<u8>, _>("properties");
        let created_at: i64 = row.get("created_at");
        let props = String::from_utf8_lossy(&properties_raw);
        let (author_type, author_name) = parse_author(&props);
        let resolved =
            props.contains("\"resolved\":\"true\"") || props.contains("\"resolved\": true");
        let status = if resolved { "resolved" } else { "pending" };

        if args.dry_run {
            info!(
                "[DRY-RUN] annotation: author={}:{} status={}",
                author_type, author_name, status
            );
            migrated += 1;
            continue;
        }

        let resolved_at: Option<i64> = if resolved {
            Some(chrono::Utc::now().timestamp())
        } else {
            None
        };

        sqlx::query(
            "INSERT INTO annotations (id, block_id, author_type, author_name, content, status, scope, created_at, resolved_at) VALUES (?, ?, ?, ?, ?, ?, 'block', ?, ?)",
        )
        .bind(&comment_id)
        .bind(&block_id)
        .bind(author_type)
        .bind(&author_name)
        .bind(&content)
        .bind(status)
        .bind(created_at)
        .bind(resolved_at)
        .execute(&pool)
        .await?;

        info!(
            "Migrated annotation: author={}:{}",
            author_type, author_name
        );
        migrated += 1;
    }

    // ── Pass 2: Fix threading for replies ──
    if !args.dry_run {
        for row in &rows {
            let comment_id: Vec<u8> = row.get("id");
            let parent_id: Option<Vec<u8>> = row.get("parent_id");

            let Some(pid) = &parent_id else { continue };

            // Check if parent comment has a corresponding annotation
            let parent_row = sqlx::query("SELECT block_id FROM annotations WHERE id = ?")
                .bind(pid)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();

            let Some(parent) = parent_row else { continue };

            let parent_block_id: Vec<u8> = parent.get("block_id");

            // Update this reply: fix block_id + set parent_annotation_id
            sqlx::query(
                "UPDATE annotations SET block_id = ?, parent_annotation_id = ? WHERE id = ?",
            )
            .bind(&parent_block_id)
            .bind(pid)
            .bind(&comment_id)
            .execute(&pool)
            .await?;

            info!("Fixed reply threading for annotation");
        }
    }

    info!(
        migrated,
        skipped_existing,
        skipped_no_parent,
        dry = args.dry_run,
        "Migration complete"
    );
    Ok(())
}

fn parse_author(properties_json: &str) -> (&'static str, String) {
    let created_by = properties_json
        .split("\"created_by\"")
        .nth(1)
        .and_then(|rest| {
            let after = rest.trim_start().strip_prefix(':').unwrap_or(rest);
            let quoted = after.trim().strip_prefix('"').unwrap_or(after);
            quoted.split('"').next()
        })
        .unwrap_or("anonymous");

    if let Some(name) = created_by.strip_prefix("user::") {
        ("human", name.to_string())
    } else if let Some(name) = created_by.strip_prefix("agent::") {
        ("agent", name.to_string())
    } else if created_by.is_empty() || created_by == "anonymous" {
        ("human", "anonymous".to_string())
    } else {
        ("human", created_by.to_string())
    }
}
