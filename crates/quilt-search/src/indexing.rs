//! Search indexing and index management

use sqlx::SqlitePool;

/// Manages the FTS5 search index.
///
/// The FTS5 index is kept in sync automatically via SQL triggers
/// on the `blocks` table. These methods are for manual maintenance:
/// full rebuilds, incremental updates, and individual block reindexing.
pub struct SearchIndex {
    pool: SqlitePool,
}

impl SearchIndex {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Refresh a single block in the FTS5 index with new content.
    ///
    /// Reads the current content from the blocks table to properly
    /// issue the FTS5 delete command (external content tables require
    /// old column values on delete), then inserts the new content.
    pub async fn index_block(&self, rowid: i64, new_content: &str) -> Result<(), String> {
        let old_content: String = sqlx::query_scalar("SELECT content FROM blocks WHERE rowid = ?")
            .bind(rowid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to read block {}: {}", rowid, e))?
            .unwrap_or_default();

        sqlx::query("INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', ?, ?)")
            .bind(rowid)
            .bind(&old_content)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete block {} from FTS index: {}", rowid, e))?;

        sqlx::query("INSERT INTO blocks_fts(rowid, content) VALUES (?, ?)")
            .bind(rowid)
            .bind(new_content)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to reindex block {}: {}", rowid, e))?;

        Ok(())
    }

    /// Remove a block from the FTS5 index by its rowid.
    ///
    /// Reads the content from the blocks table first since external
    /// content FTS5 tables require old column values for the delete command.
    pub async fn remove_block(&self, rowid: i64) -> Result<(), String> {
        let content: String = sqlx::query_scalar("SELECT content FROM blocks WHERE rowid = ?")
            .bind(rowid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to read block {}: {}", rowid, e))?
            .unwrap_or_default();

        sqlx::query("INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', ?, ?)")
            .bind(rowid)
            .bind(&content)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to remove block {} from FTS index: {}", rowid, e))?;

        Ok(())
    }

    /// Full FTS5 index rebuild.
    ///
    /// Uses the `rebuild` command on the FTS5 virtual table, which
    /// re-reads all content from the external content table (`blocks`).
    pub async fn rebuild(&self) -> Result<(), String> {
        sqlx::query("INSERT INTO blocks_fts(blocks_fts) VALUES('rebuild')")
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to rebuild FTS index: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE blocks (
                id BLOB PRIMARY KEY NOT NULL,
                page_id BLOB NOT NULL,
                parent_id BLOB,
                content TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE VIRTUAL TABLE blocks_fts USING fts5(
                content,
                content=blocks,
                content_rowid=rowid
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TRIGGER blocks_ai AFTER INSERT ON blocks BEGIN
                INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
            END",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TRIGGER blocks_ad AFTER DELETE ON blocks BEGIN
                INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            END"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TRIGGER blocks_au AFTER UPDATE ON blocks BEGIN
                INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
                INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
            END"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_index_block() {
        let pool = setup_db().await;
        let index = SearchIndex::new(pool.clone());

        let id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let page_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO blocks (id, page_id, content, created_at, updated_at) VALUES (?, ?, ?, 0, 0)")
            .bind(&id)
            .bind(&page_id)
            .bind("original content")
            .execute(&pool)
            .await
            .unwrap();

        let rowid: i64 = sqlx::query_scalar("SELECT rowid FROM blocks WHERE id = ?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();

        // Reindex with new content
        index.index_block(rowid, "updated content").await.unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM blocks_fts WHERE blocks_fts MATCH ?")
                .bind("\"updated\"")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1, "Should find the updated content via FTS");
    }

    #[tokio::test]
    async fn test_remove_block() {
        let pool = setup_db().await;
        let index = SearchIndex::new(pool.clone());

        let id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let page_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO blocks (id, page_id, content, created_at, updated_at) VALUES (?, ?, ?, 0, 0)")
            .bind(&id)
            .bind(&page_id)
            .bind("remove me")
            .execute(&pool)
            .await
            .unwrap();

        let rowid: i64 = sqlx::query_scalar("SELECT rowid FROM blocks WHERE id = ?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();

        index.remove_block(rowid).await.unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM blocks_fts WHERE blocks_fts MATCH ?")
                .bind("\"remove\"")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0, "Block should be removed from FTS index");
    }

    #[tokio::test]
    async fn test_rebuild() {
        let pool = setup_db().await;
        let index = SearchIndex::new(pool.clone());

        // Insert a block (triggers will auto-populate FTS)
        let id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let page_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO blocks (id, page_id, content, created_at, updated_at) VALUES (?, ?, ?, 0, 0)")
            .bind(&id)
            .bind(&page_id)
            .bind("rebuild test")
            .execute(&pool)
            .await
            .unwrap();

        // Rebuild re-reads from external content table
        index.rebuild().await.unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM blocks_fts WHERE blocks_fts MATCH ?")
                .bind("\"rebuild\"")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1, "Rebuild should re-index all blocks");
    }
}
