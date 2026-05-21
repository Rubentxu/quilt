//! Argument Cartographer Engine
//!
//! Main implementation of the argument mapping logic.

use crate::ai_client::{AIClient, AIClientError};
use crate::argument_cartographer::types::*;
use quilt_domain::entities::Block;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::Uuid;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::instrument;

/// The Argument Cartographer analyzes a page's blocks to build argument graphs
/// showing the logical structure of claims, evidence, and rebuttals.
#[derive(Clone)]
#[allow(dead_code)]
pub struct ArgumentCartographer {
    block_repo: Arc<dyn BlockRepository>,
    ai_client: Arc<dyn AIClient>,
}

impl std::fmt::Debug for ArgumentCartographer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArgumentCartographer")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("ai_client", &"Arc<dyn AIClient>")
            .finish()
    }
}

impl ArgumentCartographer {
    /// Create a new ArgumentCartographer.
    pub fn new(block_repo: Arc<dyn BlockRepository>, ai_client: Arc<dyn AIClient>) -> Self {
        Self {
            block_repo,
            ai_client,
        }
    }

    /// Map all arguments in a page.
    ///
    /// Builds a complete argument graph for the given page by:
    /// 1. Fetching all blocks on the page
    /// 2. Classifying each block as Claim/Evidence/Rebuttal/etc via AI
    /// 3. Building typed edges from block references
    /// 4. Computing consensus zones and node positions
    #[instrument(skip(self))]
    pub async fn map_arguments(
        &self,
        page_id: Uuid,
    ) -> Result<ArgumentGraph, ArgumentCartographerError> {
        let blocks = self
            .block_repo
            .get_by_page(page_id)
            .await
            .map_err(ArgumentCartographerError::Repository)?;

        if blocks.is_empty() {
            return Ok(ArgumentGraph {
                page_id,
                nodes: Vec::new(),
                edges: Vec::new(),
                consensus_zones: Vec::new(),
            });
        }

        // Phase 1: Classify each block
        let detections = self.detect_arguments(&blocks).await;
        let nodes = self.build_nodes(&blocks, &detections);

        // Phase 2: Build argument edges from block refs
        let edges = self.build_edges(&blocks, &nodes);

        // Phase 3: Detect consensus zones
        let consensus_zones = self.detect_consensus_zones(&nodes, &edges);

        Ok(ArgumentGraph {
            page_id,
            nodes,
            edges,
            consensus_zones,
        })
    }

    /// Detect arguments in a set of blocks via AI analysis.
    async fn detect_arguments(&self, blocks: &[Block]) -> Vec<ArgumentDetection> {
        let mut detections = Vec::new();

        for block in blocks {
            let detection = self.classify_block(block).await;
            if detection.confidence >= 0.5 {
                detections.push(detection);
            }
        }

        detections
    }

    /// Classify a single block's argument role via AI.
    async fn classify_block(&self, block: &Block) -> ArgumentDetection {
        // Simple heuristic + AI approach for now
        let content = block.content.as_plain_text();

        // Quick heuristic pre-filter to avoid AI call for obvious non-arguments
        let (classification, confidence) = self.ai_classify_block(&content).await;

        ArgumentDetection {
            classification,
            confidence,
            evidence_refs: block.refs.clone(),
        }
    }

    /// Use AI to classify a block's argument role.
    async fn ai_classify_block(&self, content: &str) -> (ArgumentRole, f64) {
        // Build a prompt for classification
        let prompt = format!(
            "Classify the following text block as one of: claim, evidence, rebuttal, qualification, assumption, or none. \
             Return ONLY a JSON object with 'role' and 'confidence' fields. \
             Example: {{\"role\": \"claim\", \"confidence\": 0.85}} \
             Text: {}",
            content.chars().take(500).collect::<String>()
        );

        let system_prompt =
            "You are an argument analysis expert. Analyze text and classify argument roles.";

        // Try AI classification first
        match self.ai_client.chat(system_prompt, &prompt).await {
            Ok(response) => {
                // Try to parse JSON response
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
                    let role_str = json.get("role").and_then(|v| v.as_str()).unwrap_or("claim");
                    let confidence = json
                        .get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.5);

                    let classification = match role_str.to_lowercase().as_str() {
                        "evidence" => ArgumentRole::Evidence,
                        "rebuttal" => ArgumentRole::Rebuttal,
                        "qualification" => ArgumentRole::Qualification,
                        "assumption" => ArgumentRole::Assumption,
                        _ => ArgumentRole::Claim,
                    };
                    return (classification, confidence);
                }
                // If JSON parsing fails, fall back to heuristic
                let classification = classify_heuristic(content);
                (classification, 0.6)
            }
            Err(_) => {
                // AI call failed, fall back to heuristic
                let classification = classify_heuristic(content);
                (classification, 0.6)
            }
        }
    }

    /// Build argument nodes from blocks and detections.
    fn build_nodes(&self, blocks: &[Block], detections: &[ArgumentDetection]) -> Vec<ArgumentNode> {
        let mut nodes = Vec::new();

        for block in blocks {
            let detection = detections
                .iter()
                .find(|d| d.evidence_refs.contains(&block.id))
                .cloned()
                .unwrap_or_else(|| ArgumentDetection {
                    classification: ArgumentRole::Claim,
                    confidence: 0.5,
                    evidence_refs: Vec::new(),
                });

            let role = if detection.confidence >= 0.5 {
                detection.classification
            } else {
                ArgumentRole::Claim
            };

            let position = compute_position(block, role);

            nodes.push(ArgumentNode {
                block_id: block.id,
                role,
                strength: detection.confidence,
                position,
            });
        }

        nodes
    }

    /// Build argument edges from block references.
    fn build_edges(&self, blocks: &[Block], nodes: &[ArgumentNode]) -> Vec<ArgumentEdge> {
        let node_ids: HashSet<_> = nodes.iter().map(|n| n.block_id).collect();
        let mut edges = Vec::new();

        for block in blocks {
            if block.refs.is_empty() {
                continue;
            }

            let source_id = block.id;

            for &target_id in &block.refs {
                if !node_ids.contains(&target_id) {
                    continue;
                }

                // Determine edge type from block content heuristics
                let edge_type = infer_edge_type(block, target_id);

                edges.push(ArgumentEdge {
                    source: source_id,
                    target: target_id,
                    edge_type,
                    confidence: 0.7, // Default confidence
                });
            }
        }

        edges
    }

    /// Detect zones of high coherence (mutual support) among nodes.
    fn detect_consensus_zones(
        &self,
        nodes: &[ArgumentNode],
        edges: &[ArgumentEdge],
    ) -> Vec<ConsensusZone> {
        if nodes.is_empty() {
            return Vec::new();
        }

        // Find groups of nodes that mutually support each other
        let mut zones = Vec::new();
        let mut visited: HashSet<Uuid> = HashSet::new();

        for node in nodes {
            if visited.contains(&node.block_id) {
                continue;
            }

            // BFS to find connected supporting cluster
            let mut zone_nodes = Vec::new();
            let mut queue = vec![node.block_id];
            let mut cluster_edges = 0;

            while let Some(current) = queue.pop() {
                if visited.contains(&current) {
                    continue;
                }
                visited.insert(current);
                zone_nodes.push(current);

                // Find all supporting edges
                for edge in edges {
                    if edge.edge_type == ArgumentEdgeType::Supports {
                        if edge.source == current && !visited.contains(&edge.target) {
                            queue.push(edge.target);
                            cluster_edges += 1;
                        }
                        if edge.target == current && !visited.contains(&edge.source) {
                            queue.push(edge.source);
                            cluster_edges += 1;
                        }
                    }
                }
            }

            if zone_nodes.len() > 1 {
                let coherence = (cluster_edges as f64 / zone_nodes.len() as f64).min(1.0);
                zones.push(ConsensusZone {
                    block_ids: zone_nodes,
                    coherence_score: coherence,
                });
            }
        }

        zones
    }

    /// Detect contradictions within a page.
    #[instrument(skip(self))]
    pub async fn detect_contradictions(
        &self,
        page_id: Uuid,
    ) -> Result<Vec<ContradictionPair>, ArgumentCartographerError> {
        let graph = self.map_arguments(page_id).await?;
        let mut contradictions = Vec::new();

        // Compare claim nodes for logical incompatibility
        let claims: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.role == ArgumentRole::Claim)
            .collect();

        for (i, a) in claims.iter().enumerate() {
            for b in claims.iter().skip(i + 1) {
                // Use AI to determine if claims are contradictory
                let conflict_type = self.assess_conflict(a.block_id, b.block_id).await;

                if let Some((conflict_type, severity)) = conflict_type {
                    contradictions.push(ContradictionPair {
                        a: a.block_id,
                        b: b.block_id,
                        conflict_type,
                        severity,
                    });
                }
            }
        }

        Ok(contradictions)
    }

    /// Assess whether two blocks are in conflict using AI.
    async fn assess_conflict(&self, _a_id: Uuid, _b_id: Uuid) -> Option<(ConflictType, f64)> {
        // For now, use a heuristic: opposite sentiment indicators
        // In a full implementation, this would use AI analysis
        // Return None for no detected conflict
        None
    }

    /// Detect logical fallacies in a block via AI analysis.
    #[instrument(skip(self))]
    pub async fn detect_fallacies(
        &self,
        block_id: Uuid,
    ) -> Result<Vec<DetectedFallacy>, ArgumentCartographerError> {
        let block = self
            .block_repo
            .get_by_id(block_id)
            .await
            .map_err(ArgumentCartographerError::Repository)?
            .ok_or(ArgumentCartographerError::BlockNotFound(block_id))?;

        // Try AI analysis, fall back to graceful degradation
        let fallacies = self.ai_detect_fallacies(&block).await;

        Ok(fallacies)
    }

    /// AI-driven fallacy detection for a block.
    async fn ai_detect_fallacies(&self, block: &Block) -> Vec<DetectedFallacy> {
        // Use AI to detect fallacies in block content
        // This is a simplified heuristic-based implementation
        let content = &block.content.to_lowercase();

        let mut fallacies = Vec::new();

        // Strawman detection: keywords indicating misrepresentation
        if content.contains("they think") && content.contains("but what they really") {
            fallacies.push(DetectedFallacy {
                fallacy_type: FallacyType::StrawMan,
                block_id: block.id,
                explanation: "Block appears to misrepresent an opponent's position".to_string(),
            });
        }

        // Ad hominem detection: personal attacks
        if content.contains("you are")
            && (content.contains("stupid")
                || content.contains("wrong")
                || content.contains("idiot"))
        {
            fallacies.push(DetectedFallacy {
                fallacy_type: FallacyType::AdHominem,
                block_id: block.id,
                explanation: "Block attacks the person rather than the argument".to_string(),
            });
        }

        // False dichotomy detection: "either/or" language
        if (content.contains("either") && content.contains("or"))
            && (content.contains("must") || content.contains("only") || content.contains("have to"))
        {
            fallacies.push(DetectedFallacy {
                fallacy_type: FallacyType::FalseDichotomy,
                block_id: block.id,
                explanation: "Block presents a false either/or choice".to_string(),
            });
        }

        // Slippery slope detection: chain of "will lead to"
        if content.contains("will lead to") && content.contains("which will then") {
            fallacies.push(DetectedFallacy {
                fallacy_type: FallacyType::SlipperySlope,
                block_id: block.id,
                explanation: "Block assumes a chain of events without justification".to_string(),
            });
        }

        // Circular reasoning: "because" near the end pointing back to claim
        let words: Vec<_> = content.split_whitespace().collect();
        if words.len() >= 10 {
            let last_third = &words[words.len() * 2 / 3..];
            let first_third = &words[..words.len() / 3];
            // Very rough heuristic: if key claim words appear at both ends
            let intersection: Vec<_> = last_third
                .iter()
                .filter(|w| first_third.contains(w) && w.len() > 5)
                .collect();
            if intersection.len() >= 2 {
                fallacies.push(DetectedFallacy {
                    fallacy_type: FallacyType::Circular,
                    block_id: block.id,
                    explanation: "Block's conclusion appears to assume its premise".to_string(),
                });
            }
        }

        fallacies
    }

    /// Score the strength of an argument (0.0–1.0).
    ///
    /// Formula: evidence_count(0.4) + ref_depth(0.3) + logical_coherence(0.3)
    /// Circular evidence chains are penalized.
    #[instrument(skip(self))]
    pub async fn score_argument_strength(
        &self,
        block_id: Uuid,
    ) -> Result<f64, ArgumentCartographerError> {
        let block = self
            .block_repo
            .get_by_id(block_id)
            .await
            .map_err(ArgumentCartographerError::Repository)?
            .ok_or(ArgumentCartographerError::BlockNotFound(block_id))?;

        let _blocks = self
            .block_repo
            .get_by_page(block.page_id)
            .await
            .map_err(ArgumentCartographerError::Repository)?;

        let graph = self.map_arguments(block.page_id).await?;

        // Find the node for this block
        let node = graph.nodes.iter().find(|n| n.block_id == block_id);

        let Some(node) = node else {
            return Ok(0.0);
        };

        // Count supporting evidence (edges pointing TO this block from evidence nodes)
        let evidence_count = graph
            .edges
            .iter()
            .filter(|e| e.target == block_id && e.edge_type == ArgumentEdgeType::Supports)
            .count();

        // Also count refs TO this block (direct references count as support)
        let direct_refs = block.refs.len();

        // Calculate ref depth (longest path to this node)
        let ref_depth = calculate_ref_depth(block_id, &graph.edges, 5);

        // Check for circular reasoning
        let has_cycle = detect_cycle(block_id, &graph.edges);

        // Coherence score from AI (placeholder heuristic)
        let logical_coherence = if has_cycle {
            node.strength * 0.5 // Penalty for circular reasoning
        } else {
            node.strength
        };

        let total_evidence = evidence_count + direct_refs;
        let evidence_score = (total_evidence as f64 * 0.1).min(0.4);
        let depth_score = (ref_depth as f64 * 0.1).min(0.3);
        let coherence_score = logical_coherence * 0.3;

        let total = evidence_score + depth_score + coherence_score;
        Ok(total.clamp(0.0, 1.0))
    }
}

/// Errors for the Argument Cartographer.
#[derive(Debug, thiserror::Error)]
pub enum ArgumentCartographerError {
    #[error("Block not found: {0}")]
    BlockNotFound(Uuid),
    #[error("Repository error: {0}")]
    Repository(#[from] DomainError),
    #[error("AI client error: {0}")]
    AI(#[from] AIClientError),
}

// ── Helper functions ───────────────────────────────────────────────────────────

/// Heuristic classification of block content into argument roles.
fn classify_heuristic(content: &str) -> ArgumentRole {
    let content_lower = content.to_lowercase();

    // Check for evidence markers
    if content_lower.contains("according to")
        || content_lower.contains("study shows")
        || content_lower.contains("data indicates")
        || content_lower.contains("benchmark")
        || content_lower.contains("%")
        || content_lower.contains("citation")
    {
        return ArgumentRole::Evidence;
    }

    // Check for rebuttal markers
    if content_lower.contains("however")
        || content_lower.contains("but")
        || content_lower.contains("contrary")
        || content_lower.contains("despite")
        || content_lower.contains("although")
    {
        return ArgumentRole::Rebuttal;
    }

    // Check for qualification markers
    if content_lower.contains("however")
        || content_lower.contains("although")
        || content_lower.contains("except")
        || content_lower.contains("unless")
    {
        return ArgumentRole::Qualification;
    }

    // Check for assumption markers
    if content_lower.contains("assume")
        || content_lower.contains("suppose")
        || content_lower.contains("presume")
        || content_lower.contains("given that")
    {
        return ArgumentRole::Assumption;
    }

    ArgumentRole::Claim
}

/// Compute the position (in/out/neutral) of a block in the argument graph.
fn compute_position(block: &Block, role: ArgumentRole) -> Position {
    let ref_count = block.refs.len();

    match role {
        ArgumentRole::Claim | ArgumentRole::Assumption => {
            if ref_count > 2 {
                Position::In
            } else {
                Position::Neutral
            }
        }
        ArgumentRole::Evidence => Position::In,
        ArgumentRole::Rebuttal => Position::Out,
        ArgumentRole::Qualification => Position::Neutral,
    }
}

/// Infer the edge type from block content heuristics.
fn infer_edge_type(block: &Block, _target_id: Uuid) -> ArgumentEdgeType {
    let content_lower = block.content.to_lowercase();

    if content_lower.contains("supports")
        || content_lower.contains("because")
        || content_lower.contains("therefore")
        || content_lower.contains("since")
    {
        ArgumentEdgeType::Supports
    } else if content_lower.contains("refutes")
        || content_lower.contains("contradicts")
        || content_lower.contains("however")
        || content_lower.contains("but")
    {
        ArgumentEdgeType::Refutes
    } else {
        ArgumentEdgeType::Qualifies
    }
}

/// Calculate the reference depth (longest path) to a node.
fn calculate_ref_depth(node_id: Uuid, edges: &[ArgumentEdge], max_depth: usize) -> usize {
    let mut depth = 0;
    let mut _current = vec![node_id];
    let mut visited = HashSet::new();

    for _ in 0..max_depth {
        let mut next = Vec::new();
        for edge in edges {
            if edge.target == node_id && !visited.contains(&edge.source) {
                next.push(edge.source);
                visited.insert(edge.source);
            }
        }
        if next.is_empty() {
            break;
        }
        _current = next;
        depth += 1;
    }

    depth
}

/// Detect if a node is part of a circular reasoning chain.
fn detect_cycle(node_id: Uuid, edges: &[ArgumentEdge]) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![node_id];

    while let Some(current) = stack.pop() {
        if visited.contains(&current) {
            return true;
        }
        visited.insert(current);

        for edge in edges {
            if edge.source == current {
                stack.push(edge.target);
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quilt_domain::value_objects::{BlockFormat, JournalDay};
    use std::collections::HashMap;

    fn make_block(id: Uuid, refs: Vec<Uuid>, page_id: Uuid, content: &str) -> Block {
        Block {
            id,
            page_id,
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: quilt_domain::content::BlockContent::from_text(content),
            properties: std::collections::HashMap::new(),
            refs,
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            journal_day: None,
            updated_journal_day: None,
        }
    }

    fn uuid_from_u8(i: u8) -> Uuid {
        let mut b = [0u8; 16];
        b[0] = i;
        Uuid::from_bytes(b)
    }

    #[derive(Debug, Clone, Default)]
    struct MockBlockRepo {
        pages: HashMap<Uuid, Vec<Block>>,
        blocks: HashMap<Uuid, Block>,
    }

    impl MockBlockRepo {
        fn from_blocks(page_id: Uuid, blocks: Vec<Block>) -> Self {
            let blocks_map: HashMap<Uuid, Block> =
                blocks.iter().map(|b| (b.id, b.clone())).collect();
            Self {
                pages: vec![(page_id, blocks)].into_iter().collect(),
                blocks: blocks_map,
            }
        }
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepo {
        async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError> {
            Ok(self.blocks.get(&id).cloned())
        }
        async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(self.pages.get(&page_id).cloned().unwrap_or_default())
        }
        async fn get_children(&self, _parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_with_refs(&self, _id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
            Err(DomainError::NotImplemented(
                "get_with_refs not implemented in mock",
            ))
        }
        async fn insert(&self, _block: &Block) -> Result<(), DomainError> {
            Ok(())
        }
        async fn update(&self, _block: &Block) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn hard_delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn restore(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_deleted_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn recycle_bin(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn move_block(
            &self,
            _id: Uuid,
            _new_parent: Option<Uuid>,
            _new_order: f64,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn count_by_page(&self, _page_id: Uuid) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn get_blocks_by_journal_day(
            &self,
            _day: JournalDay,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_orphan_blocks(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_map_arguments_empty_page() {
        let page_id = uuid_from_u8(1);
        let repo = Arc::new(MockBlockRepo::default());
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let graph = cartographer.map_arguments(page_id).await.unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[tokio::test]
    async fn test_map_arguments_linear_chain() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let c = uuid_from_u8(12);

        let blocks = vec![
            make_block(a, vec![b], page_id, "Rust is a great language"),
            make_block(b, vec![c], page_id, "Because it has memory safety"),
            make_block(c, vec![], page_id, "According to a study"),
        ];

        let repo = Arc::new(MockBlockRepo::from_blocks(page_id, blocks.clone()));
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let graph = cartographer.map_arguments(page_id).await.unwrap();
        assert_eq!(graph.nodes.len(), 3);
        // Edges: a→b, b→c
        assert_eq!(graph.edges.len(), 2);
    }

    #[tokio::test]
    async fn test_detect_contradictions_no_claims() {
        let page_id = uuid_from_u8(1);
        let block = make_block(uuid_from_u8(10), vec![], page_id, "Just a note");
        let repo = Arc::new(MockBlockRepo::from_blocks(page_id, vec![block]));
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let contradictions = cartographer.detect_contradictions(page_id).await.unwrap();
        assert!(contradictions.is_empty());
    }

    #[tokio::test]
    async fn test_detect_fallacies_no_fallacy() {
        let page_id = uuid_from_u8(1);
        let block_id = uuid_from_u8(10);
        let block = make_block(
            block_id,
            vec![],
            page_id,
            "Rust has ownership and borrowing",
        );

        let repo = Arc::new(MockBlockRepo::from_blocks(page_id, vec![block]));
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let fallacies = cartographer.detect_fallacies(block_id).await.unwrap();
        // No obvious fallacy indicators in this content
        assert!(fallacies.is_empty());
    }

    #[tokio::test]
    async fn test_detect_fallacies_strawman() {
        let page_id = uuid_from_u8(1);
        let block_id = uuid_from_u8(10);
        let block = make_block(
            block_id,
            vec![],
            page_id,
            "They think Rust is too complex but what they really mean is they don't understand it",
        );

        let repo = Arc::new(MockBlockRepo::from_blocks(page_id, vec![block]));
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let fallacies = cartographer.detect_fallacies(block_id).await.unwrap();
        assert!(!fallacies.is_empty());
        assert!(fallacies
            .iter()
            .any(|f| f.fallacy_type == FallacyType::StrawMan));
    }

    #[tokio::test]
    async fn test_score_argument_strength_unsupported() {
        let page_id = uuid_from_u8(1);
        let block_id = uuid_from_u8(10);
        let block = make_block(block_id, vec![], page_id, "Rust is the best");

        let repo = Arc::new(MockBlockRepo::from_blocks(page_id, vec![block]));
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let score = cartographer
            .score_argument_strength(block_id)
            .await
            .unwrap();
        // Unsupported claim should score < 0.3
        assert!(score < 0.3);
    }

    #[tokio::test]
    async fn test_score_argument_strength_well_supported() {
        let page_id = uuid_from_u8(1);
        let claim = uuid_from_u8(10);
        let evidence1 = uuid_from_u8(11);
        let evidence2 = uuid_from_u8(12);
        let evidence3 = uuid_from_u8(13);

        let blocks = vec![
            make_block(
                claim,
                vec![evidence1, evidence2, evidence3],
                page_id,
                "Rust has great performance",
            ),
            make_block(
                evidence1,
                vec![],
                page_id,
                "Study shows Rust is as fast as C++",
            ),
            make_block(
                evidence2,
                vec![],
                page_id,
                "According to benchmarks, Rust scores highly",
            ),
            make_block(
                evidence3,
                vec![],
                page_id,
                "Data from production shows low latency",
            ),
        ];

        let repo = Arc::new(MockBlockRepo::from_blocks(page_id, blocks.clone()));
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let score = cartographer.score_argument_strength(claim).await.unwrap();
        // With 3 evidence refs, score should be > 0.3
        assert!(score >= 0.3, "score was {}", score);
    }

    #[tokio::test]
    async fn test_consensus_zone_detection() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let c = uuid_from_u8(12);

        let blocks = vec![
            make_block(
                a,
                vec![b],
                page_id,
                "Rust is memory safe because of ownership",
            ),
            make_block(
                b,
                vec![c],
                page_id,
                "Ownership prevents data races because it enforces rules",
            ),
            make_block(
                c,
                vec![a],
                page_id,
                "Data races are prevented by the ownership system",
            ),
        ];

        let repo = Arc::new(MockBlockRepo::from_blocks(page_id, blocks.clone()));
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        let cartographer = ArgumentCartographer::new(repo, ai);

        let graph = cartographer.map_arguments(page_id).await.unwrap();
        // Should detect at least one consensus zone (a→b→c→a cycle)
        assert!(!graph.consensus_zones.is_empty());
    }

    #[test]
    fn test_classify_heuristic_evidence() {
        let content = "According to a recent study, Rust is faster than Go";
        assert_eq!(classify_heuristic(content), ArgumentRole::Evidence);
    }

    #[test]
    fn test_classify_heuristic_claim() {
        let content = "I think Rust is the best programming language";
        assert_eq!(classify_heuristic(content), ArgumentRole::Claim);
    }

    #[test]
    fn test_classify_heuristic_rebuttal() {
        let content = "However, Rust has a steep learning curve";
        assert_eq!(classify_heuristic(content), ArgumentRole::Rebuttal);
    }
}
