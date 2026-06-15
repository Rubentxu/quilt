//! Integration tests for `SqliteAnnotationRepository`.
//!
//! Covers the CRUD contract, filtering, thread replies, and cascade
//! delete on block delete. Uses an in-memory SQLite database with
//! the full migration suite applied.

use sqlx::SqlitePool;

use quilt_domain::entities::{
    Annotation, AnnotationCreate, AnnotationScope, AnnotationStatus, AuthorType,
};
use quilt_domain::repositories::{
    AnnotationFilters, AnnotationRepository, AnnotationRepositoryExt,
};
use quilt_domain::value_objects::Uuid;
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::SqliteAnnotationRepository;

/// Connect to an in-memory SQLite, run migrations, return the pool.
async fn setup_test_db() -> SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory DB");
    connection::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}

/// Insert a minimal page + block. Returns the block id. Required
/// because annotations FK to `blocks(id)`.
///
/// Uses a unique page name per call so multiple inserts in the same
/// in-memory DB don't trip the `pages.name` UNIQUE constraint.
async fn insert_parent_block(pool: &SqlitePool) -> Uuid {
    let page_id = Uuid::new_v4();
    let block_id = Uuid::new_v4();
    let page_name = format!("page-{}", page_id.short());
    let now: i64 = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
                 VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.as_bytes().to_vec())
    .bind(&page_name)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO blocks (id, page_id, format, block_type, content, properties, \
                 collapsed, created_at, updated_at, refs, tags) \
                 VALUES (?, ?, 'markdown', 'paragraph', '', '{}', 0, ?, ?, '[]', '[]')",
    )
    .bind(block_id.as_bytes().to_vec())
    .bind(page_id.as_bytes().to_vec())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
    block_id
}

fn make_block_annotation(block_id: Uuid, content: &str, author: &str) -> Annotation {
    Annotation::new(AnnotationCreate {
        block_id,
        scope: AnnotationScope::Block,
        author_type: AuthorType::Human,
        author_name: author.to_string(),
        content: content.to_string(),
        parent_annotation_id: None,
        highlight_start: None,
        highlight_end: None,
    })
    .expect("block annotation should be valid")
}

fn make_inline_annotation(block_id: Uuid, content: &str, start: u32, end: u32) -> Annotation {
    Annotation::new(AnnotationCreate {
        block_id,
        scope: AnnotationScope::Inline,
        author_type: AuthorType::Agent,
        author_name: "claude".to_string(),
        content: content.to_string(),
        parent_annotation_id: None,
        highlight_start: Some(start),
        highlight_end: Some(end),
    })
    .expect("inline annotation should be valid")
}

#[tokio::test]
async fn insert_and_get_by_id_roundtrips() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let ann = make_block_annotation(block_id, "first!", "alice");
    let id = ann.id;
    repo.insert(&ann).await.expect("insert");

    let loaded = repo.get_by_id(id).await.expect("get").expect("must exist");
    assert_eq!(loaded.id, id);
    assert_eq!(loaded.block_id, block_id);
    assert_eq!(loaded.content, "first!");
    assert_eq!(loaded.author_name, "alice");
    assert_eq!(loaded.scope, AnnotationScope::Block);
    assert_eq!(loaded.status, AnnotationStatus::Pending);
    assert_eq!(loaded.highlight_start, None);
    assert_eq!(loaded.highlight_end, None);
}

#[tokio::test]
async fn get_by_block_returns_all_for_block() {
    // We don't pin the exact order here because `Annotation::new()`
    // sets `created_at = Utc::now()` and timestamp storage rounds to
    // epoch seconds — annotations created in the same second have the
    // same `created_at` and the order is determined by the secondary
    // key (`id ASC`, i.e. UUID lexicographic order). What matters is
    // that the call returns all annotations targeting the block, and
    // doesn't include annotations for other blocks.
    let pool = setup_test_db().await;
    let block_a = insert_parent_block(&pool).await;
    let block_b = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    repo.insert(&make_block_annotation(block_a, "a1", "u"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_a, "a2", "u"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_b, "b1", "u"))
        .await
        .unwrap();

    let loaded = repo.get_by_block(block_a).await.unwrap();
    assert_eq!(loaded.len(), 2);
    for a in &loaded {
        assert_eq!(a.block_id, block_a);
    }
}

#[tokio::test]
async fn update_persists_changes() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let mut ann = make_block_annotation(block_id, "original", "u1");
    repo.insert(&ann).await.unwrap();
    ann.resolve("u2".to_string());
    repo.update(&ann).await.unwrap();

    let loaded = repo.get_by_id(ann.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, AnnotationStatus::Resolved);
    assert_eq!(loaded.resolved_by.as_deref(), Some("u2"));
    assert!(loaded.resolved_at.is_some());
}

#[tokio::test]
async fn delete_removes_row() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let ann = make_block_annotation(block_id, "x", "u");
    repo.insert(&ann).await.unwrap();
    repo.delete(ann.id).await.unwrap();
    let loaded = repo.get_by_id(ann.id).await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn delete_idempotent_on_missing_id() {
    let pool = setup_test_db().await;
    let repo = SqliteAnnotationRepository::new(pool.clone());
    // Deleting a non-existent id should be a no-op (no error).
    let result = repo.delete(Uuid::new_v4()).await;
    assert!(result.is_ok(), "delete of missing id must be a no-op");
}

#[tokio::test]
async fn get_by_status_filters_correctly() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let mut a1 = make_block_annotation(block_id, "p1", "u");
    let mut a2 = make_block_annotation(block_id, "p2", "u");
    let a3 = make_block_annotation(block_id, "p3", "u");
    a1.set_in_progress();
    a2.resolve("agent".to_string());
    repo.insert(&a1).await.unwrap();
    repo.insert(&a2).await.unwrap();
    repo.insert(&a3).await.unwrap();

    let pending = repo.get_by_status("pending").await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, a3.id);

    let in_progress = repo.get_by_status("in_progress").await.unwrap();
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].id, a1.id);

    let resolved = repo.get_by_status("resolved").await.unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].id, a2.id);
}

#[tokio::test]
async fn get_by_author_filters_correctly() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    repo.insert(&make_block_annotation(block_id, "a", "alice"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_id, "b", "bob"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_id, "c", "alice"))
        .await
        .unwrap();

    let alice = repo.get_by_author("alice").await.unwrap();
    assert_eq!(alice.len(), 2);
    for a in &alice {
        assert_eq!(a.author_name, "alice");
    }
}

#[tokio::test]
async fn get_root_annotations_excludes_replies() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let parent = make_block_annotation(block_id, "parent", "u");
    let reply = Annotation::new(AnnotationCreate {
        block_id,
        scope: AnnotationScope::Block,
        author_type: AuthorType::Agent,
        author_name: "agent".to_string(),
        content: "reply".to_string(),
        parent_annotation_id: Some(parent.id),
        highlight_start: None,
        highlight_end: None,
    })
    .unwrap();
    repo.insert(&parent).await.unwrap();
    repo.insert(&reply).await.unwrap();

    let roots = repo.get_root_annotations().await.unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, parent.id);
}

#[tokio::test]
async fn get_thread_replies_returns_only_children() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let parent = make_block_annotation(block_id, "parent", "u");
    let r1 = Annotation::new(AnnotationCreate {
        block_id,
        scope: AnnotationScope::Block,
        author_type: AuthorType::Agent,
        author_name: "agent".to_string(),
        content: "r1".to_string(),
        parent_annotation_id: Some(parent.id),
        highlight_start: None,
        highlight_end: None,
    })
    .unwrap();
    let r2 = Annotation::new(AnnotationCreate {
        block_id,
        scope: AnnotationScope::Block,
        author_type: AuthorType::Agent,
        author_name: "agent".to_string(),
        content: "r2".to_string(),
        parent_annotation_id: Some(parent.id),
        highlight_start: None,
        highlight_end: None,
    })
    .unwrap();
    let unrelated = make_block_annotation(block_id, "other", "u");
    repo.insert(&parent).await.unwrap();
    repo.insert(&unrelated).await.unwrap();
    repo.insert(&r1).await.unwrap();
    repo.insert(&r2).await.unwrap();

    let replies = repo.get_thread_replies(parent.id).await.unwrap();
    assert_eq!(replies.len(), 2);
    let ids: Vec<Uuid> = replies.iter().map(|a| a.id).collect();
    assert!(ids.contains(&r1.id));
    assert!(ids.contains(&r2.id));
}

#[tokio::test]
async fn inline_offsets_roundtrip() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let ann = make_inline_annotation(block_id, "underline this", 5, 10);
    let id = ann.id;
    repo.insert(&ann).await.unwrap();

    let loaded = repo.get_by_id(id).await.unwrap().unwrap();
    assert_eq!(loaded.scope, AnnotationScope::Inline);
    assert_eq!(loaded.highlight_start, Some(5));
    assert_eq!(loaded.highlight_end, Some(10));
}

#[tokio::test]
async fn cascade_delete_on_block() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    repo.insert(&make_block_annotation(block_id, "a", "u"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_id, "b", "u"))
        .await
        .unwrap();
    assert_eq!(repo.get_by_block(block_id).await.unwrap().len(), 2);

    // Delete the parent block — annotations must cascade.
    sqlx::query("DELETE FROM blocks WHERE id = ?")
        .bind(block_id.as_bytes().to_vec())
        .execute(&pool)
        .await
        .unwrap();
    assert!(repo.get_by_block(block_id).await.unwrap().is_empty());
}

#[tokio::test]
async fn get_by_filters_empty_returns_all() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    repo.insert(&make_block_annotation(block_id, "a", "u1"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_id, "b", "u2"))
        .await
        .unwrap();

    let all = repo
        .get_by_filters(&AnnotationFilters::default())
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn get_by_filters_block_id() {
    let pool = setup_test_db().await;
    let block_a = insert_parent_block(&pool).await;
    let block_b = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    repo.insert(&make_block_annotation(block_a, "a", "u"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_b, "b", "u"))
        .await
        .unwrap();

    let a_only = repo
        .get_by_filters(&AnnotationFilters::default().with_block_id(block_a))
        .await
        .unwrap();
    assert_eq!(a_only.len(), 1);
    assert_eq!(a_only[0].block_id, block_a);
}

#[tokio::test]
async fn get_by_filters_status_and_scope_combined() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    // pending + block
    repo.insert(&make_block_annotation(block_id, "p-b", "u"))
        .await
        .unwrap();
    // resolved + block
    let mut a2 = make_block_annotation(block_id, "r-b", "u");
    a2.resolve("u".to_string());
    repo.insert(&a2).await.unwrap();
    // pending + inline
    repo.insert(&make_inline_annotation(block_id, "p-i", 0, 1))
        .await
        .unwrap();

    let f = AnnotationFilters::default()
        .with_status("pending")
        .with_scope(AnnotationScope::Block);
    let res = repo.get_by_filters(&f).await.unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].content, "p-b");
}

#[tokio::test]
async fn get_by_filters_author_name() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    repo.insert(&make_block_annotation(block_id, "a", "alice"))
        .await
        .unwrap();
    repo.insert(&make_block_annotation(block_id, "b", "bob"))
        .await
        .unwrap();

    let f = AnnotationFilters::default().with_author_name("alice");
    let res = repo.get_by_filters(&f).await.unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].author_name, "alice");
}

#[tokio::test]
async fn open_annotations_returns_pending_and_in_progress() {
    let pool = setup_test_db().await;
    let block_id = insert_parent_block(&pool).await;
    let repo = SqliteAnnotationRepository::new(pool.clone());

    let mut a1 = make_block_annotation(block_id, "p", "u");
    let mut a2 = make_block_annotation(block_id, "ip", "u");
    let mut a3 = make_block_annotation(block_id, "r", "u");
    a1.set_in_progress();
    a2.resolve("u".to_string());
    a3.dismiss();
    repo.insert(&a1).await.unwrap();
    repo.insert(&a2).await.unwrap();
    repo.insert(&a3).await.unwrap();

    let open = repo.get_open_annotations().await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].content, "p");

    let terminal = repo.get_terminal_annotations().await.unwrap();
    assert_eq!(terminal.len(), 2);
}
