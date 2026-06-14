//! Graph use cases — the Graph Lens V1 endpoint's BFS logic, lifted
//! out of the HTTP handler so the algorithm is reusable from any
//! transport (REST, MCP, CLI).
//!
//! # Lens semantics
//!
//! A *lens* is a BFS over two kinds of edges:
//! 1. `parent → child` (a block's `parent_id` is the source, the
//!    child is the target).
//! 2. `forward-ref → target` (a block's `refs[]` list).
//!
//! The BFS is depth-bounded (`depth` in `1..=3` inclusive). A `property:`
//! focus is single-hop by definition (no shared structure between
//! matched blocks).
//!
//! Auth is enforced by the HTTP middleware; the use case trusts the
//! caller and returns whatever the graph contains.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::Block;
use quilt_domain::repositories::{BlockQueryRepository, BlockRepository, PageRepository};
use quilt_domain::value_objects::Uuid;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tracing::instrument;

/// Inclusive lower bound for the `depth` parameter.
pub const MIN_DEPTH: u32 = 1;
/// Inclusive upper bound for the `depth` parameter.
pub const MAX_DEPTH: u32 = 3;

/// Parsed focus selector. The HTTP layer converts the wire `focus=...`
/// string into one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    /// `block:<uuid>` — a specific block.
    Block(Uuid),
    /// `page:<name>` — all root blocks of the page.
    Page(String),
    /// `property:<key>` — every block with this property key.
    Property(String),
}

impl Focus {
    /// Parse a focus string. Returns `Ok(None)` when the input is
    /// absent or empty (semantic "no focus"). Returns
    /// `ApplicationError::Validation` for any input that does not
    /// start with a recognized prefix.
    pub fn parse(s: &str) -> Result<Option<Self>, ApplicationError> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(None);
        }
        if let Some(rest) = s.strip_prefix("block:") {
            let uuid = Uuid::parse_str(rest.trim())
                .ok_or_else(|| ApplicationError::Validation(format!("Invalid block UUID in focus: '{}'", rest)))?;
            return Ok(Some(Focus::Block(uuid)));
        }
        if let Some(rest) = s.strip_prefix("page:") {
            let name = rest.trim();
            if name.is_empty() {
                return Err(ApplicationError::Validation(
                    "Page focus requires a non-empty page name".to_string(),
                ));
            }
            return Ok(Some(Focus::Page(name.to_string())));
        }
        if let Some(rest) = s.strip_prefix("property:") {
            let key = rest.trim();
            if key.is_empty() {
                return Err(ApplicationError::Validation(
                    "Property focus requires a non-empty key".to_string(),
                ));
            }
            return Ok(Some(Focus::Property(key.to_string())));
        }
        Err(ApplicationError::Validation(format!(
            "Unknown focus prefix in '{}'. Expected one of: block:<uuid>, page:<name>, property:<key>",
            s
        )))
    }
}

/// Edge kind in a lens — `"parent-child"` or `"ref"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Parent → child (via `parent_id`).
    ParentChild,
    /// Forward ref → target (via the in-memory `RefIndex`).
    Ref,
}

impl EdgeKind {
    /// Wire representation (matches the previous DTO's `kind` field).
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeKind::ParentChild => "parent-child",
            EdgeKind::Ref => "ref",
        }
    }
}

/// A node in the lens.
#[derive(Debug, Clone)]
pub struct LensNode {
    /// Block UUID.
    pub id: Uuid,
    /// Truncated content preview (first 200 chars).
    pub content: String,
    /// Page UUID the block lives on.
    pub page_id: Uuid,
    /// Page name the block lives on.
    pub page_name: String,
    /// Whether the host page is a journal page.
    pub is_journal: bool,
    /// Whether the block has any properties set.
    pub has_properties: bool,
}

/// An edge in the lens.
#[derive(Debug, Clone)]
pub struct LensEdge {
    /// Source block UUID.
    pub from: Uuid,
    /// Target block UUID.
    pub to: Uuid,
    /// Edge kind — see [`EdgeKind`].
    pub kind: EdgeKind,
}

/// Response returned by [`GraphUseCases::lens`].
#[derive(Debug, Clone, Default)]
pub struct LensResult {
    /// All blocks included in the lens, in BFS-discovery order.
    pub nodes: Vec<LensNode>,
    /// All parent→child and ref edges whose endpoints are both in
    /// `nodes`.
    pub edges: Vec<LensEdge>,
}

/// Forward-ref lookup closure. Production wires this to
/// `RefService::get_forward_refs`; tests can pass a static map.
pub type RefLookup<'a> = Box<dyn Fn(Uuid) -> Vec<Uuid> + Send + Sync + 'a>;

/// Use cases for graph queries.
#[async_trait]
pub trait GraphUseCases: Send + Sync {
    /// Compute a lens (BFS subgraph) centred on the given focus.
    ///
    /// `depth` must be in `1..=3`; an out-of-range value yields
    /// `ApplicationError::Validation`. `ref_lookup` is a closure
    /// the BFS calls to enumerate a block's forward-ref targets —
    /// the caller (e.g. the HTTP layer) wires this to its in-memory
    /// `RefIndex`.
    async fn lens(
        &self,
        focus: Option<&Focus>,
        depth: u32,
        ref_lookup: RefLookup<'_>,
    ) -> Result<LensResult, ApplicationError>;
}

/// Implementation of [`GraphUseCases`] backed by generic repositories.
pub struct GraphUseCasesImpl<BR: BlockRepository + BlockQueryRepository, PR: PageRepository> {
    block_repo: Arc<BR>,
    page_repo: Arc<PR>,
}

impl<BR: BlockRepository + BlockQueryRepository, PR: PageRepository> GraphUseCasesImpl<BR, PR> {
    /// Create a new use-case instance.
    pub fn new(block_repo: Arc<BR>, page_repo: Arc<PR>) -> Self {
        Self {
            block_repo,
            page_repo,
        }
    }
}

#[async_trait]
impl<BR: BlockRepository + BlockQueryRepository + 'static, PR: PageRepository + 'static> GraphUseCases
    for GraphUseCasesImpl<BR, PR>
{
    #[instrument(skip(self, ref_lookup))]
    async fn lens(
        &self,
        focus: Option<&Focus>,
        depth: u32,
        ref_lookup: RefLookup<'_>,
    ) -> Result<LensResult, ApplicationError> {
        // 1. Validate depth.
        if !(MIN_DEPTH..=MAX_DEPTH).contains(&depth) {
            return Err(ApplicationError::Validation(format!(
                "depth must be between {} and {}",
                MIN_DEPTH, MAX_DEPTH
            )));
        }

        // 2. No focus → empty graph.
        let Some(focus) = focus else {
            return Ok(LensResult::default());
        };

        // 3. Resolve the focus into seed blocks + a page-metadata map.
        let (seed_blocks, seed_pages, effective_depth) = match focus {
            Focus::Block(uuid) => {
                let Some(block) = self
                    .block_repo
                    .get_by_id(*uuid)
                    .await
                    .map_err(ApplicationError::Domain)?
                else {
                    return Ok(LensResult::default());
                };
                let mut pages = HashMap::new();
                if let Some(p) = self
                    .page_repo
                    .get_by_id(block.page_id)
                    .await
                    .map_err(ApplicationError::Domain)?
                {
                    pages.insert(block.page_id, (p.name, p.journal));
                }
                (vec![block], pages, depth)
            }
            Focus::Page(name) => {
                let Some(page) = self
                    .page_repo
                    .get_by_name(name)
                    .await
                    .map_err(ApplicationError::Domain)?
                else {
                    return Ok(LensResult::default());
                };
                let roots = self
                    .block_repo
                    .get_by_page(page.id)
                    .await
                    .map_err(ApplicationError::Domain)?;
                let pages = HashMap::from([(page.id, (page.name.clone(), page.journal))]);
                (roots, pages, depth)
            }
            Focus::Property(key) => {
                let blocks = self
                    .block_repo
                    .list_by_property_key(key, 0)
                    .await
                    .map_err(ApplicationError::Domain)?;
                let mut pages: HashMap<Uuid, (String, bool)> = HashMap::new();
                let mut seen_pages: HashSet<Uuid> = HashSet::new();
                for b in &blocks {
                    if seen_pages.insert(b.page_id) {
                        if let Some(p) = self
                            .page_repo
                            .get_by_id(b.page_id)
                            .await
                            .map_err(ApplicationError::Domain)?
                        {
                            pages.insert(b.page_id, (p.name, p.journal));
                        }
                    }
                }
                (blocks, pages, 1)
            }
        };

        // 4. Run the BFS.
        let mut blocks_map: HashMap<Uuid, Block> = HashMap::new();
        let mut edges: Vec<(Uuid, Uuid, EdgeKind)> = Vec::new();
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut queue: VecDeque<(Uuid, u32)> = VecDeque::new();
        let mut page_meta = seed_pages;

        for seed in &seed_blocks {
            if visited.insert(seed.id) {
                blocks_map.insert(seed.id, seed.clone());
                queue.push_back((seed.id, 0));
            }
        }

        while let Some((current, dist)) = queue.pop_front() {
            if dist >= effective_depth.saturating_sub(1) {
                continue;
            }

            // Parent → child edges
            let children = self
                .block_repo
                .get_children(current)
                .await
                .map_err(ApplicationError::Domain)
                .unwrap_or_default();
            for child in children {
                let child_id = child.id;
                if !page_meta.contains_key(&child.page_id) {
                    if let Ok(Some(p)) = self.page_repo.get_by_id(child.page_id).await {
                        page_meta.insert(child.page_id, (p.name, p.journal));
                    }
                }
                if !visited.contains(&child_id) {
                    visited.insert(child_id);
                    blocks_map.insert(child_id, child.clone());
                    edges.push((current, child_id, EdgeKind::ParentChild));
                    queue.push_back((child_id, dist + 1));
                } else {
                    edges.push((current, child_id, EdgeKind::ParentChild));
                }
            }

            // Forward refs
            for target in ref_lookup(current) {
                if let Ok(Some(target_block)) = self.block_repo.get_by_id(target).await {
                    let target_id = target_block.id;
                    if !page_meta.contains_key(&target_block.page_id) {
                        if let Ok(Some(p)) = self.page_repo.get_by_id(target_block.page_id).await {
                            page_meta.insert(target_block.page_id, (p.name, p.journal));
                        }
                    }
                    if !visited.contains(&target_id) {
                        visited.insert(target_id);
                        blocks_map.insert(target_id, target_block.clone());
                        edges.push((current, target_id, EdgeKind::Ref));
                        queue.push_back((target_id, dist + 1));
                    } else {
                        edges.push((current, target_id, EdgeKind::Ref));
                    }
                }
            }
        }

        // 5. Deduplicate edges.
        let mut seen_edges: HashSet<(Uuid, Uuid, EdgeKind)> = HashSet::new();
        edges.retain(|e| seen_edges.insert(*e));
        // Drop edges whose endpoints are not in the node set.
        edges.retain(|(from, to, _)| blocks_map.contains_key(from) && blocks_map.contains_key(to));

        // 6. Convert to DTOs.
        let nodes: Vec<LensNode> = blocks_map
            .values()
            .map(|b| {
                let (page_name, is_journal) = page_meta
                    .get(&b.page_id)
                    .cloned()
                    .unwrap_or_else(|| (String::new(), false));
                LensNode {
                    id: b.id,
                    content: truncate(&b.content, 200),
                    page_id: b.page_id,
                    page_name,
                    is_journal,
                    has_properties: !b.properties.is_empty(),
                }
            })
            .collect();

        let edges = edges
            .into_iter()
            .map(|(from, to, kind)| LensEdge { from, to, kind })
            .collect();

        Ok(LensResult { nodes, edges })
    }
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
