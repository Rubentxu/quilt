//! Integration tests for `BlockRepository::list_distinct_keys` against
//! a real SQLite database (T5 of the property-keys-endpoint change).
//!
//! The unit tests in `quilt-test-helpers` cover the *contract* — sort
//! order, cursor strict-greater-than, limit slicing. These tests cover
//! the *SQL* — that `json_each(properties)` actually flattens the
//! JSON BLOB and that the storage-layer behavior matches the contract
//! the unit tests pin down.
//!
//! Strategy: seed blocks with hand-rolled JSON property blobs (bypassing
//! the repo's `Block::properties` JSON encoder) so the test owns the
//! serialized shape. This catches encoding drift if someone ever changes
//! how `properties` is serialized.

use quilt_domain::entities::{Block, BlockCreate, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository,
};
use std::collections::HashMap;
use url::Url;

async fn setup_test_db() -> sqlx::SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory DB");
    connection::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}

async fn insert_page(pool: &sqlx::SqlitePool, name: &str) -> Uuid {
    let page = quilt_domain::entities::Page::new(PageCreate {
        name: name.to_string(),
        title: None,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: HashMap::new(),
    })
    .unwrap();
    SqlitePageRepository::new(pool.clone())
        .insert(&page)
        .await
        .unwrap();
    page.id
}

/// Build a block with the given typed properties — the repo's normal
/// `Block::properties` HashMap path.
fn make_block_with_properties(
    page_id: Uuid,
    content: &str,
    properties: HashMap<String, PropertyValue>,
) -> Block {
    Block::new(BlockCreate {
        page_id,
        content: content.to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        properties,
    })
    .unwrap()
}

fn props_str(pairs: &[(&str, &str)]) -> HashMap<String, PropertyValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), PropertyValue::string(*v)))
        .collect()
}

#[tokio::test]
async fn list_distinct_keys_empty_db() {
    let pool = setup_test_db().await;
    let repo = SqliteBlockRepository::new(pool);
    let keys = repo.list_distinct_keys(None, 50).await.unwrap();
    assert!(keys.is_empty(), "empty DB should yield no keys");
}

#[tokio::test]
async fn list_distinct_keys_blocks_with_empty_properties() {
    // All blocks with `properties = '{}'` — no top-level keys.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "empty-props").await;
    let repo = SqliteBlockRepository::new(pool);

    let b1 = make_block_with_properties(page_id, "x", HashMap::new());
    let b2 = make_block_with_properties(page_id, "y", HashMap::new());
    repo.insert(&b1).await.unwrap();
    repo.insert(&b2).await.unwrap();

    let keys = repo.list_distinct_keys(None, 50).await.unwrap();
    assert!(
        keys.is_empty(),
        "blocks with empty properties should yield no keys, got: {keys:?}"
    );
}

#[tokio::test]
async fn list_distinct_keys_returns_distinct_keys_sorted_asc() {
    // Seed blocks with overlapping keys → DISTINCT must dedupe.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "sorted").await;
    let repo = SqliteBlockRepository::new(pool);

    let b1 = make_block_with_properties(
        page_id,
        "b1",
        props_str(&[("status", "Doing"), ("priority", "A")]),
    );
    let b2 = make_block_with_properties(
        page_id,
        "b2",
        props_str(&[("status", "Done"), ("deadline", "2026-01-01")]),
    );
    let b3 = make_block_with_properties(page_id, "b3", props_str(&[("alpha", "x")]));
    repo.insert(&b1).await.unwrap();
    repo.insert(&b2).await.unwrap();
    repo.insert(&b3).await.unwrap();

    let keys = repo.list_distinct_keys(None, 50).await.unwrap();
    assert_eq!(
        keys,
        vec![
            "alpha".to_string(),
            "deadline".to_string(),
            "priority".to_string(),
            "status".to_string(),
        ]
    );
}

#[tokio::test]
async fn list_distinct_keys_cursor_paginates_strictly_greater() {
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "cursor").await;
    let repo = SqliteBlockRepository::new(pool);

    // 5 blocks, each with one distinct key a, b, c, d, e.
    for (i, k) in ["a", "b", "c", "d", "e"].iter().enumerate() {
        let block = make_block_with_properties(page_id, &format!("b{i}"), props_str(&[(k, "v")]));
        repo.insert(&block).await.unwrap();
    }

    // cursor = "b" → strictly c, d, e
    let keys = repo.list_distinct_keys(Some("b"), 50).await.unwrap();
    assert_eq!(
        keys,
        vec!["c".to_string(), "d".to_string(), "e".to_string()],
        "cursor must be strictly greater than"
    );

    // cursor at first key returns the rest.
    let keys = repo.list_distinct_keys(Some("a"), 50).await.unwrap();
    assert_eq!(
        keys,
        vec![
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string()
        ]
    );

    // cursor past the end → empty page
    let keys = repo.list_distinct_keys(Some("zzz"), 50).await.unwrap();
    assert!(keys.is_empty(), "cursor past end should be empty page");
}

#[tokio::test]
async fn list_distinct_keys_limit_honored() {
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "limit").await;
    let repo = SqliteBlockRepository::new(pool);

    // 30 distinct keys.
    for i in 0..30 {
        let key = format!("k{i:02}");
        let block =
            make_block_with_properties(page_id, &format!("b{i}"), props_str(&[(&key, "v")]));
        repo.insert(&block).await.unwrap();
    }

    // limit=10 → exactly 10 smallest keys
    let keys = repo.list_distinct_keys(None, 10).await.unwrap();
    assert_eq!(keys.len(), 10);
    assert_eq!(keys[0], "k00");
    assert_eq!(keys[9], "k09");

    // limit=50 → all 30 (no error on limit > total)
    let keys = repo.list_distinct_keys(None, 50).await.unwrap();
    assert_eq!(keys.len(), 30);
}

#[tokio::test]
async fn list_distinct_keys_top_level_only_does_not_dive_into_nested_values() {
    // `json_each` of an OBJECT yields its top-level keys. A nested
    // object value (e.g. `{"nested": {"foo": 1}}`) must NOT contribute
    // "foo" as a top-level key — only "nested" appears.
    //
    // The domain `PropertyValue` enum doesn't currently support nested
    // objects, so we hand-roll a raw JSON blob on the row after the
    // standard `repo.insert()` call (which writes `{}` as the
    // `properties` BLOB). This isolates the storage-layer SQL from
    // the domain-layer serializer.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "nested").await;
    let repo = SqliteBlockRepository::new(pool.clone());

    // Standard insert (writes `properties = '{}'`).
    let mut block = make_block_with_properties(page_id, "x", HashMap::new());
    repo.insert(&block).await.unwrap();

    // Now overwrite the `properties` blob with a hand-rolled JSON
    // object that contains a NESTED value. SQLite's `json_each` on
    // an object yields one row per top-level key — it does NOT
    // recurse into nested objects.
    let props_json = r#"{"top": 1, "nested": {"foo": 1, "bar": 2}, "alpha": "x"}"#;
    let id_bytes: Vec<u8> = block.id.as_bytes().to_vec();
    sqlx::query("UPDATE blocks SET properties = ? WHERE id = ?")
        .bind(props_json.as_bytes())
        .bind(&id_bytes)
        .execute(&pool)
        .await
        .unwrap();

    // Sanity: the standard read path should still load the block
    // (the nested object simply doesn't deserialize into a typed
    // `PropertyValue`).
    let _loaded = repo.get_by_id(block.id).await.unwrap().unwrap();
    block.id = Uuid::nil(); // suppress unused warning

    let keys = repo.list_distinct_keys(None, 50).await.unwrap();
    assert_eq!(
        keys,
        vec!["alpha".to_string(), "nested".to_string(), "top".to_string()],
        "nested object keys must NOT be promoted to top-level (got: {keys:?})"
    );
}

#[tokio::test]
async fn list_distinct_keys_cursor_with_special_characters() {
    // A key containing a `/` (real-world: "priority/level"). The
    // query uses `WHERE je.key > ?` and binds by parameter, so
    // special chars are not interpreted by SQLite.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "special").await;
    let repo = SqliteBlockRepository::new(pool);

    let b1 = make_block_with_properties(
        page_id,
        "x",
        props_str(&[("priority/level", "high"), ("status", "Doing")]),
    );
    let b2 = make_block_with_properties(page_id, "y", props_str(&[("priority/level", "low")]));
    repo.insert(&b1).await.unwrap();
    repo.insert(&b2).await.unwrap();

    let keys = repo.list_distinct_keys(None, 50).await.unwrap();
    assert_eq!(
        keys,
        vec!["priority/level".to_string(), "status".to_string()]
    );

    // Cursor at "priority/level" → only "status" remains.
    let keys = repo
        .list_distinct_keys(Some("priority/level"), 50)
        .await
        .unwrap();
    assert_eq!(keys, vec!["status".to_string()]);
}

// ── ADR-0027: typed PropertyValue SQLite blob round-trip ───────────────────────

#[tokio::test]
async fn sqlite_block_properties_url_value_serializes_to_string() {
    // ADR-0027: Url variant serializes to a JSON string; read back via raw SQL.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "url-blob").await;
    let repo = SqliteBlockRepository::new(pool.clone());

    let url = url::Url::parse("https://quilt.dev").unwrap();
    let block = make_block_with_properties(
        page_id,
        "has URL",
        vec![
            ("source-url".to_string(), PropertyValue::url(url)),
        ]
        .into_iter()
        .collect(),
    );
    repo.insert(&block).await.unwrap();

    // Read the raw JSON blob via SQL
    let id_bytes: Vec<u8> = block.id.as_bytes().to_vec();
    let row: (Vec<u8>,) = sqlx::query_as("SELECT properties FROM blocks WHERE id = ?")
        .bind(&id_bytes)
        .fetch_one(&pool)
        .await
        .unwrap();

    let json_str = String::from_utf8(row.0).unwrap();
    assert!(
        json_str.contains("https://quilt.dev"),
        "blob should contain URL string: {}",
        json_str
    );
}

#[tokio::test]
async fn sqlite_block_properties_naive_date_value_serializes_to_string() {
    // ADR-0027: NaiveDate variant serializes to YYYY-MM-DD string.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "date-blob").await;
    let repo = SqliteBlockRepository::new(pool.clone());

    let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    let block = make_block_with_properties(
        page_id,
        "has date",
        vec![("scheduled".to_string(), PropertyValue::naive_date(d))]
            .into_iter()
            .collect(),
    );
    repo.insert(&block).await.unwrap();

    // Read the raw JSON blob via SQL
    let id_bytes: Vec<u8> = block.id.as_bytes().to_vec();
    let row: (Vec<u8>,) = sqlx::query_as("SELECT properties FROM blocks WHERE id = ?")
        .bind(&id_bytes)
        .fetch_one(&pool)
        .await
        .unwrap();

    let json_str = String::from_utf8(row.0).unwrap();
    assert!(
        json_str.contains("2026-06-15"),
        "blob should contain ISO date string: {}",
        json_str
    );
}

#[tokio::test]
async fn sqlite_block_properties_url_value_reads_back_as_string() {
    // ADR-0027: Lossy round-trip — Url stored, comes back as String.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "url-lossy").await;
    let repo = SqliteBlockRepository::new(pool.clone());

    let url = url::Url::parse("https://quilt.dev").unwrap();
    let block = make_block_with_properties(
        page_id,
        "has URL",
        vec![("source-url".to_string(), PropertyValue::url(url))]
            .into_iter()
            .collect(),
    );
    repo.insert(&block).await.unwrap();

    // Read back via repo — property should be String (lossy round-trip)
    let loaded = repo.get_by_id(block.id).await.unwrap().unwrap();
    let prop = loaded.properties.get("source-url");
    assert!(
        matches!(prop, Some(PropertyValue::String(s)) if s.contains("https://quilt.dev")),
        "Url should round-trip as String, got: {:?}",
        prop
    );
}

#[tokio::test]
async fn sqlite_block_properties_naive_date_value_reads_back_as_string() {
    // ADR-0027: Lossy round-trip — NaiveDate stored, comes back as String.
    let pool = setup_test_db().await;
    let page_id = insert_page(&pool, "date-lossy").await;
    let repo = SqliteBlockRepository::new(pool.clone());

    let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    let block = make_block_with_properties(
        page_id,
        "has date",
        vec![("scheduled".to_string(), PropertyValue::naive_date(d))]
            .into_iter()
            .collect(),
    );
    repo.insert(&block).await.unwrap();

    // Read back via repo — property should be String (lossy round-trip)
    let loaded = repo.get_by_id(block.id).await.unwrap().unwrap();
    let prop = loaded.properties.get("scheduled");
    assert!(
        matches!(prop, Some(PropertyValue::String(s)) if s == "2026-06-15"),
        "NaiveDate should round-trip as String, got: {:?}",
        prop
    );
}
