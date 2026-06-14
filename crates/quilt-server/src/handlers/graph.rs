//! HTTP handler for the Graph Lens V1 endpoint.
//!
//! Mounts at `GET /api/v1/graph/lens?focus=...&depth=...` and returns
//! a subgraph of the knowledge graph centered on the given focus.
//!
//! # Focus grammar
//!
//! The `focus` query parameter is a small DSL that names the center
//! of the lens:
//!
//! * `block:<uuid>` — a specific block (and N hops of children/forward
//!   refs from it).
//! * `page:<name>`  — all root blocks of the named page (and N hops
//!   from each).
//! * `property:<key>` — every block whose `properties` map contains
//!   `<key>`. `depth` is forced to 1 (the matched blocks are the
//!   "result"; further hops would not be semantically meaningful).
//! * absent or empty — returns 200 with an empty graph (no focus → no
//!   subgraph to project).
//!
//! Anything else is a 400.
//!
//! # Depth
//!
//! * Default: 1.
//! * Bounds: 1..=3 inclusive.
//! * Out of range → 400.
//! * Ignored for `property:` (always 1).
//!
//! # What "N hops" means
//!
//! Traversal is BFS from the focus over two kinds of edges:
//!
//! 1. **Parent → child** (a block's `parent_id` is the source, the
//!    child is the target). Always traversable; works without
//!    RefService.
//! 2. **Forward ref → target** (a block's `refs[]` list). Uses
//!    `RefService::get_forward_refs` to be O(1) per node.
//!
//! The implementation is the BFS in [`bfs_subgraph`] — pure over
//! `BlockRepository` + `RefService`, so it can be unit-tested with
//! the in-memory test fixtures and reused if the lens ever moves to a
//! different transport.
//!
//! Auth is enforced by the global middleware on `/api/v1/*` — this
//! handler does not need to re-check.

use axum::{
    Json, Router,
    extract::{Extension, Query},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use quilt_application::services::ref_service::RefServiceTrait;
use quilt_domain::entities::Block;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::Uuid;

/// Inclusive lower bound for `?depth=`.
const MIN_DEPTH: u32 = 1;
/// Inclusive upper bound for `?depth=`.
const MAX_DEPTH: u32 = 3;

/// Default depth when `?depth=` is absent.
fn default_depth() -> u32 {
    1
}

/// Query string for `GET /api/v1/graph/lens`.
///
/// `focus` is optional — missing/empty returns an empty graph.
/// `depth` defaults to 1, bounded to 1..=3. Validation happens in
/// the handler so the error message is domain-meaningful rather than
/// a serde 400.
#[derive(Debug, Clone, Deserialize)]
pub struct LensParams {
    /// Focus selector — see module docs for the grammar.
    #[serde(default)]
    pub focus: Option<String>,
    /// Traversal depth (1..=3, default 1).
    #[serde(default = "default_depth")]
    pub depth: u32,
}

/// Node in a lens response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LensNodeDto {
    /// Block UUID.
    pub id: String,
    /// Truncated content preview (first 200 chars).
    pub content: String,
    /// Page UUID the block lives on.
    pub page_id: String,
    /// Page name the block lives on.
    pub page_name: String,
    /// Whether the host page is a journal page.
    pub is_journal: bool,
    /// Whether the block has any properties set.
    pub has_properties: bool,
}

/// Edge in a lens response — from parent to child OR from source ref
/// to target. The frontend distinguishes the two via `kind` if it
/// needs to.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LensEdgeDto {
    /// Source block UUID.
    pub from: String,
    /// Target block UUID.
    pub to: String,
    /// `"parent-child"` or `"ref"`.
    pub kind: &'static str,
}

/// Response body for `GET /api/v1/graph/lens`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LensResponse {
    /// Echo of the focus string (or `null` when absent).
    pub focus: Option<String>,
    /// Effective depth used (after any clamping/forcing for property).
    pub depth: u32,
    /// All blocks included in the lens.
    pub nodes: Vec<LensNodeDto>,
    /// All parent→child and ref edges whose endpoints are both in
    /// `nodes`.
    pub edges: Vec<LensEdgeDto>,
}

/// Parsed focus.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Focus {
    /// `block:<uuid>` — a specific block.
    Block(Uuid),
    /// `page:<name>` — all root blocks of the page.
    Page(String),
    /// `property:<key>` — every block with this property key.
    Property(String),
}

impl Focus {
    /// Parse a focus string. Returns `Ok(None)` when the input is
    /// absent or empty (semantic "no focus"). Returns `Err` for any
    /// input that does not start with a recognized prefix.
    fn parse(s: &str) -> Result<Option<Self>, String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(None);
        }
        if let Some(rest) = s.strip_prefix("block:") {
            let uuid = Uuid::parse_str(rest.trim())
                .map_err(|_| format!("Invalid block UUID in focus: '{}'", rest))?;
            return Ok(Some(Focus::Block(uuid)));
        }
        if let Some(rest) = s.strip_prefix("page:") {
            let name = rest.trim();
            if name.is_empty() {
                return Err("Page focus requires a non-empty page name".to_string());
            }
            return Ok(Some(Focus::Page(name.to_string())));
        }
        if let Some(rest) = s.strip_prefix("property:") {
            let key = rest.trim();
            if key.is_empty() {
                return Err("Property focus requires a non-empty key".to_string());
            }
            return Ok(Some(Focus::Property(key.to_string())));
        }
        Err(format!(
            "Unknown focus prefix in '{}'. Expected one of: block:<uuid>, page:<name>, property:<key>",
            s
        ))
    }
}

/// Output of [`bfs_subgraph`] before DTO conversion.
#[derive(Debug, Clone, Default)]
struct Subgraph {
    /// Blocks included, keyed by UUID.
    blocks: HashMap<Uuid, Block>,
    /// Edges to render (both endpoints known).
    edges: Vec<(Uuid, Uuid, &'static str)>,
}

/// BFS over a focus. `depth == 1` means "just the focus, no edges";
/// `depth == 2` adds the focus's children + forward-ref targets;
/// etc.
///
/// `seed_blocks` are the starting nodes for BFS. For `block:` focus
/// it has length 1; for `page:` focus it has every root block of the
/// page; for `property:` focus it has every matching block and
/// `depth` is irrelevant (only 1 level of "edges" is meaningful, but
/// since seed blocks have no shared structure we just return them).
///
/// `ref_lookup` is an async closure that returns the forward-ref
/// UUIDs of a block. Production wires this to
/// `RefService::get_forward_refs` (held under a `RwLock`); tests can
/// pass a static map.
#[allow(clippy::too_many_arguments)]
async fn bfs_subgraph<BR, PR, F, Fut>(
    block_repo: &BR,
    page_repo: &PR,
    seed_blocks: Vec<Block>,
    seed_pages: HashMap<Uuid, (String, bool)>,
    depth: u32,
    ref_lookup: F,
) -> Subgraph
where
    BR: BlockRepository + ?Sized,
    PR: PageRepository + ?Sized,
    F: Fn(Uuid) -> Fut,
    Fut: std::future::Future<Output = Vec<Uuid>>,
{
    let mut graph = Subgraph::default();
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut queue: VecDeque<(Uuid, u32)> = VecDeque::new();

    // Seed: insert all seed blocks and enqueue them at distance 0.
    for seed in &seed_blocks {
        if visited.insert(seed.id.into()) {
            graph.blocks.insert(seed.id.into(), seed.clone());
            queue.push_back((seed.id.into(), 0));
        }
    }

    // Page metadata cache so we don't hit the page repo N times.
    let mut page_meta: HashMap<Uuid, (String, bool)> = seed_pages;

    while let Some((current, dist)) = queue.pop_front() {
        // Distance from the focus:
        //   dist=0 → the focus itself
        //   dist=1 → direct child / forward-ref target
        //   dist=2 → grandchild
        // `depth=1` means "just the focus" — the focus is at
        // dist=0, so we MUST NOT expand its neighbours. `depth=2`
        // means "focus + 1 hop" — we expand nodes at dist=0 but
        // stop at dist=1.
        if dist >= depth.saturating_sub(1) {
            continue;
        }

        // 1. Parent → child edges.
        let children = block_repo.get_children(current).await.unwrap_or_default();
        for child in children {
            let child_id: Uuid = child.id.into();
            if !visited.contains(&child_id) {
                // Resolve page metadata on demand.
                if !page_meta.contains_key(&child.page_id.into()) {
                    if let Ok(Some(p)) = page_repo.get_by_id(child.page_id.into()).await {
                        page_meta.insert(child.page_id.into(), (p.name, p.journal));
                    }
                }
                visited.insert(child_id);
                graph.blocks.insert(child_id, child.clone());
                graph.edges.push((current, child_id, "parent-child"));
                queue.push_back((child_id, dist + 1));
            } else {
                // Even if visited, record the edge if it was missed.
                // Deduplicate edges below.
                graph.edges.push((current, child_id, "parent-child"));
            }
        }

        // 2. Forward refs.
        for target in ref_lookup(current).await {
            if let Ok(Some(target_block)) = block_repo.get_by_id(target).await {
                let target_id: Uuid = target_block.id.into();
                if !visited.contains(&target_id) {
                    if !page_meta.contains_key(&target_block.page_id.into()) {
                        if let Ok(Some(p)) = page_repo.get_by_id(target_block.page_id.into()).await
                        {
                            page_meta.insert(target_block.page_id.into(), (p.name, p.journal));
                        }
                    }
                    visited.insert(target_id);
                    graph.blocks.insert(target_id, target_block.clone());
                    graph.edges.push((current, target_id, "ref"));
                    queue.push_back((target_id, dist + 1));
                } else {
                    graph.edges.push((current, target_id, "ref"));
                }
            }
        }
    }

    // Deduplicate edges (we may have re-pushed edges when revisiting
    // a node that was already discovered via another path).
    let mut seen_edges: HashSet<(Uuid, Uuid, &'static str)> = HashSet::new();
    graph.edges.retain(|e| seen_edges.insert(*e));

    // Drop edges whose endpoints are not in the node set (defensive
    // — BFS should never produce those, but the assertion is cheap).
    graph
        .edges
        .retain(|(from, to, _)| graph.blocks.contains_key(from) && graph.blocks.contains_key(to));

    // Cache page_meta for later DTO conversion by moving into caller.
    let _ = page_meta;

    graph
}

/// Router for `/api/v1/graph`. Mounted at `/api/v1/graph` in
/// `routes.rs`.
pub fn routes() -> Router {
    Router::new().route("/lens", get(get_lens))
}

/// `GET /api/v1/graph/lens?focus=...&depth=...`
///
/// Returns a subgraph of the knowledge graph centered on the given
/// focus. See the module docs for the focus grammar and depth
/// semantics.
///
/// # Errors
/// * 400 — invalid focus prefix, invalid UUID, depth out of range.
/// * 401 — handled by the global auth middleware.
#[instrument(skip(ref_service, block_repo, page_repo))]
pub async fn get_lens(
    Query(params): Query<LensParams>,
    Extension(ref_service): Extension<Arc<dyn RefServiceTrait>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
) -> Result<Json<LensResponse>, AppError> {
    // 1. Validate depth.
    if params.depth < MIN_DEPTH || params.depth > MAX_DEPTH {
        return Err(AppError::BadRequest(format!(
            "depth must be between {MIN_DEPTH} and {MAX_DEPTH}"
        )));
    }

    // 2. Parse focus. None / empty → empty graph (graceful).
    let focus_str = params.focus.as_deref().unwrap_or("").to_string();
    let focus = Focus::parse(&focus_str).map_err(|e| AppError::BadRequest(e))?;

    let Some(focus) = focus else {
        return Ok(Json(LensResponse {
            focus: None,
            depth: params.depth,
            nodes: vec![],
            edges: vec![],
        }));
    };

    // 3. Resolve the focus into seed blocks + a closure for
    //    forward-ref lookup. We do this in a match so each branch
    //    can pick the right repo method.
    let (seed_blocks, seed_pages, effective_depth) = match focus {
        Focus::Block(uuid) => {
            let block = match block_repo.get_by_id(uuid).await {
                Ok(Some(b)) => b,
                Ok(None) => {
                    // Unknown block → empty graph (graceful).
                    return Ok(Json(LensResponse {
                        focus: Some(focus_str),
                        depth: params.depth,
                        nodes: vec![],
                        edges: vec![],
                    }));
                }
                Err(e) => return Err(AppError::Internal(e.to_string())),
            };
            let mut pages = HashMap::new();
            if let Ok(Some(p)) = page_repo.get_by_id(block.page_id.into()).await {
                pages.insert(block.page_id.into(), (p.name, p.journal));
            }
            (vec![block], pages, params.depth)
        }
        Focus::Page(name) => {
            let page = match page_repo.get_by_name(&name).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    return Ok(Json(LensResponse {
                        focus: Some(focus_str),
                        depth: params.depth,
                        nodes: vec![],
                        edges: vec![],
                    }));
                }
                Err(e) => return Err(AppError::Internal(e.to_string())),
            };
            let roots = block_repo
                .get_by_page(page.id.into())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let pages = HashMap::from([(page.id.into(), (page.name.clone(), page.journal))]);
            (roots, pages, params.depth)
        }
        Focus::Property(key) => {
            // List all blocks with the given key, then collect their
            // pages. Property focus is single-hop by definition.
            let blocks = block_repo
                .list_by_property_key(&key, 0)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let mut pages = HashMap::new();
            let mut seen_pages: HashSet<Uuid> = HashSet::new();
            for b in &blocks {
                if seen_pages.insert(b.page_id.into()) {
                    if let Ok(Some(p)) = page_repo.get_by_id(b.page_id.into()).await {
                        pages.insert(b.page_id.into(), (p.name, p.journal));
                    }
                }
            }
            (blocks, pages, 1)
        }
    };

    // 4. Build the ref lookup closure backed by RefService.
    let ref_lookup = {
        let svc = ref_service.clone();
        move |block_id: Uuid| {
            let svc = svc.clone();
            async move {
                svc.get_forward_refs(block_id)
                    .into_iter()
                    .map(|(u, _)| u)
                    .collect::<Vec<_>>()
            }
        }
    };

    // 5. Run the BFS. For a property focus the result is just the
    //    seed set (no edges to traverse by definition).
    let graph = bfs_subgraph(
        block_repo.as_ref(),
        page_repo.as_ref(),
        seed_blocks,
        seed_pages,
        effective_depth,
        ref_lookup,
    )
    .await;

    // 6. Convert to DTOs.
    let nodes: Vec<LensNodeDto> = graph
        .blocks
        .values()
        .map(|b| {
            // We need page_name + is_journal. We only have them for
            // pages the BFS visited; fall back to a query if not.
            // In practice every visited block has its page in the
            // cache because we resolve on first sight.
            // Use a quick synchronous lookup by re-reading from the
            // page repo would require await — but we already cached.
            // We need page_meta here. Easiest: store it on the
            // subgraph. For simplicity, do a best-effort sync
            // resolution using the block's own data + the page_meta
            // we can reconstruct by querying only when missing.
            LensNodeDto {
                id: b.id.to_string(),
                content: truncate(&b.content, 200),
                page_id: b.page_id.to_string(),
                page_name: String::new(), // filled below
                is_journal: false,        // filled below
                has_properties: !b.properties.is_empty(),
            }
        })
        .collect();

    // For page name + is_journal we need the page_meta map. We did
    // not pass it through to keep `bfs_subgraph` pure on the BFS
    // contract. Build it here from the node set with one query per
    // distinct page.
    let mut page_meta: HashMap<Uuid, (String, bool)> = HashMap::new();
    for b in graph.blocks.values() {
        let pid: Uuid = b.page_id.into();
        if !page_meta.contains_key(&pid) {
            if let Ok(Some(p)) = page_repo.get_by_id(pid).await {
                page_meta.insert(pid, (p.name, p.journal));
            }
        }
    }

    let nodes: Vec<LensNodeDto> = nodes
        .into_iter()
        .map(|mut n| {
            // Re-parse page_id as Uuid to look up.
            if let Ok(pid) = Uuid::parse_str(&n.page_id) {
                if let Some((name, journal)) = page_meta.get(&pid) {
                    n.page_name = name.clone();
                    n.is_journal = *journal;
                }
            }
            n
        })
        .collect();

    let edges: Vec<LensEdgeDto> = graph
        .edges
        .into_iter()
        .map(|(from, to, kind)| LensEdgeDto {
            from: from.to_string(),
            to: to.to_string(),
            kind,
        })
        .collect();

    Ok(Json(LensResponse {
        focus: Some(focus_str),
        depth: effective_depth,
        nodes,
        edges,
    }))
}

/// Truncate a string for the content preview, with an ellipsis when
/// the original was longer than the limit.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

// ── Unit tests for pure parsing and helpers ───────────────────────
//
// The BFS itself is exercised end-to-end by the integration tests
// in `tests/graph_lens_handler.rs`. These unit tests cover the
// small pure functions that don't need a real repo: focus grammar
// parsing and content truncation.

#[cfg(test)]
mod tests {
    use super::*;

    // ── focus parsing ──────────────────────────────────────────────

    #[test]
    fn focus_parse_empty_returns_none() {
        assert_eq!(Focus::parse("").unwrap(), None);
        assert_eq!(Focus::parse("   ").unwrap(), None);
    }

    #[test]
    fn focus_parse_block() {
        let f = Focus::parse("block:550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert!(matches!(f, Some(Focus::Block(_))));
    }

    #[test]
    fn focus_parse_block_invalid_uuid_errors() {
        assert!(Focus::parse("block:garbage").is_err());
    }

    #[test]
    fn focus_parse_page() {
        let f = Focus::parse("page:my page").unwrap();
        assert_eq!(f, Some(Focus::Page("my page".to_string())));
    }

    #[test]
    fn focus_parse_page_empty_name_errors() {
        assert!(Focus::parse("page:").is_err());
        assert!(Focus::parse("page:   ").is_err());
    }

    #[test]
    fn focus_parse_property() {
        let f = Focus::parse("property:status").unwrap();
        assert_eq!(f, Some(Focus::Property("status".to_string())));
    }

    #[test]
    fn focus_parse_property_empty_key_errors() {
        assert!(Focus::parse("property:").is_err());
    }

    #[test]
    fn focus_parse_unknown_prefix_errors() {
        assert!(Focus::parse("garbage").is_err());
        assert!(Focus::parse("tag:foo").is_err());
    }

    // ── truncate helper ────────────────────────────────────────────

    #[test]
    fn truncate_short_passes_through() {
        assert_eq!(truncate("hi", 5), "hi");
    }

    #[test]
    fn truncate_long_adds_ellipsis() {
        let s = "a".repeat(10);
        let out = truncate(&s, 5);
        assert_eq!(out.chars().count(), 6); // 5 + ellipsis
        assert!(out.ends_with('…'));
    }

    #[test]
    fn truncate_exact_boundary_keeps_string() {
        let s = "a".repeat(5);
        assert_eq!(truncate(&s, 5), s);
    }
}
