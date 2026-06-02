//! Force-directed graph simulation (pure Rust, no WASM dependencies)
//!
//! Implements a spring-mass force simulation with:
//! - Repulsion between all node pairs (Coulomb's law)
//! - Attraction along edges (Hooke's law)
//! - Center gravity to prevent drift
//! - Velocity damping for convergence
//!
//! # Architecture
//!
//! This module is a pure algorithm with **zero framework dependencies**.
//! It compiles on WASM and native targets alike. The only external crate
//! is `serde` for serialization, which is optional-gated in tests.
//!
//! # Usage
//!
//! ```rust
//! use quilt_core::graph::force_simulation::ForceSimulation;
//!
//! let mut sim = ForceSimulation::new(
//!     vec!["a".into(), "b".into()],
//!     vec!["Node A".into(), "Node B".into()],
//!     vec![false, false],
//!     vec![0],
//!     vec![1],
//! );
//! sim.run();
//! assert!(sim.is_converged());
//! ```

use serde::{Deserialize, Serialize};

/// A node in the force simulation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimNode {
    pub id: String,
    pub name: String,
    pub journal: bool,
    /// Current position
    pub x: f64,
    pub y: f64,
    /// Velocity
    vx: f64,
    vy: f64,
    /// Radius based on connection count
    pub radius: f64,
    /// Whether the node is pinned (fixed position)
    pub pinned: bool,
    /// Number of connections (used for sizing)
    pub connection_count: usize,
}

impl SimNode {
    /// Create a new simulation node at a random position near origin
    pub fn new(id: String, name: String, journal: bool, connection_count: usize) -> Self {
        // Random initial position in a circle
        let angle = rand_angle();
        let r = rand_in_range(50.0, 150.0);
        Self {
            id,
            name,
            journal,
            x: angle.cos() * r,
            y: angle.sin() * r,
            vx: 0.0,
            vy: 0.0,
            radius: Self::compute_radius(connection_count),
            pinned: false,
            connection_count,
        }
    }

    /// Compute radius from connection count: log scale, clamped to [8, 30]
    fn compute_radius(connections: usize) -> f64 {
        let base = (connections as f64 + 1.0).log10() * 10.0;
        base.clamp(8.0, 30.0)
    }

    /// Pin the node at its current position
    pub fn pin(&mut self) {
        self.pinned = true;
        self.vx = 0.0;
        self.vy = 0.0;
    }

    /// Unpin the node (let it move again)
    pub fn unpin(&mut self) {
        self.pinned = false;
    }

    /// Move the node to a new position (for drag interaction)
    pub fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    /// Apply a force to this node
    pub fn apply_force(&mut self, fx: f64, fy: f64) {
        if self.pinned {
            return;
        }
        self.vx += fx;
        self.vy += fy;
    }

    /// Update position based on velocity, apply damping
    pub fn integrate(&mut self, damping: f64) {
        if self.pinned {
            return;
        }
        self.x += self.vx;
        self.y += self.vy;
        self.vx *= damping;
        self.vy *= damping;
    }

    /// Total kinetic energy (for convergence check)
    pub fn kinetic_energy(&self) -> f64 {
        if self.pinned {
            return 0.0;
        }
        self.vx * self.vx + self.vy * self.vy
    }
}

/// An edge in the force simulation (index-based for speed)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimEdge {
    /// Index of source node
    pub source_idx: usize,
    /// Index of target node
    pub target_idx: usize,
}

/// Configuration parameters for the force simulation.
///
/// These control the physical forces and convergence behavior.
/// All fields have sensible defaults — supply only what you need to override.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationParams {
    /// Repulsion strength (Coulomb constant) — default: 500.0
    pub repulsion: Option<f64>,
    /// Attraction strength (Spring constant) — default: 0.05
    pub attraction: Option<f64>,
    /// Rest length for springs — default: 100.0
    pub rest_length: Option<f64>,
    /// Damping factor applied each integration step — default: 0.85
    pub damping: Option<f64>,
    /// Center gravity strength — default: 0.01
    pub gravity: Option<f64>,
    /// Velocity threshold for convergence — default: 0.5
    pub convergence_threshold: Option<f64>,
    /// Maximum iterations before forced stop — default: 500
    pub max_iterations: Option<usize>,
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            repulsion: Some(500.0),
            attraction: Some(0.05),
            rest_length: Some(100.0),
            damping: Some(0.85),
            gravity: Some(0.01),
            convergence_threshold: Some(0.5),
            max_iterations: Some(500),
        }
    }
}

/// The force simulation state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForceSimulation {
    nodes: Vec<SimNode>,
    edges: Vec<SimEdge>,
    /// Repulsion strength (Coulomb constant)
    repulsion: f64,
    /// Attraction strength (Spring constant)
    attraction: f64,
    /// Rest length for springs
    rest_length: f64,
    /// Damping factor applied each integration
    damping: f64,
    /// Center gravity strength
    gravity: f64,
    /// Velocity threshold for convergence
    convergence_threshold: f64,
    /// Maximum iterations before forced stop
    max_iterations: usize,
}

impl ForceSimulation {
    /// Create a new simulation from graph data
    pub fn new(
        ids: Vec<String>,
        names: Vec<String>,
        journals: Vec<bool>,
        sources: Vec<usize>,
        targets: Vec<usize>,
    ) -> Self {
        // Count connections per node
        let mut connection_counts = vec![0usize; ids.len()];
        for &s in &sources {
            if s < ids.len() {
                connection_counts[s] += 1;
            }
        }
        for &t in &targets {
            if t < ids.len() {
                connection_counts[t] += 1;
            }
        }

        let nodes: Vec<SimNode> = ids
            .into_iter()
            .zip(names)
            .zip(journals)
            .zip(connection_counts)
            .map(|(((id, name), journal), count)| SimNode::new(id, name, journal, count))
            .collect();

        let edges: Vec<SimEdge> = sources
            .into_iter()
            .zip(targets)
            .map(|(s, t)| SimEdge {
                source_idx: s,
                target_idx: t,
            })
            .collect();

        Self {
            nodes,
            edges,
            repulsion: 500.0,
            attraction: 0.05,
            rest_length: 100.0,
            damping: 0.85,
            gravity: 0.01,
            convergence_threshold: 0.5,
            max_iterations: 500,
        }
    }

    /// Create a new simulation with custom parameters
    pub fn with_params(
        ids: Vec<String>,
        names: Vec<String>,
        journals: Vec<bool>,
        sources: Vec<usize>,
        targets: Vec<usize>,
        params: SimulationParams,
    ) -> Self {
        let mut sim = Self::new(ids, names, journals, sources, targets);
        if let Some(v) = params.repulsion {
            sim.repulsion = v;
        }
        if let Some(v) = params.attraction {
            sim.attraction = v;
        }
        if let Some(v) = params.rest_length {
            sim.rest_length = v;
        }
        if let Some(v) = params.damping {
            sim.damping = v;
        }
        if let Some(v) = params.gravity {
            sim.gravity = v;
        }
        if let Some(v) = params.convergence_threshold {
            sim.convergence_threshold = v;
        }
        if let Some(v) = params.max_iterations {
            sim.max_iterations = v;
        }
        sim
    }

    /// Number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get a reference to a node by index
    pub fn node(&self, idx: usize) -> Option<&SimNode> {
        self.nodes.get(idx)
    }

    /// Get a mutable reference to a node by index
    pub fn node_mut(&mut self, idx: usize) -> Option<&mut SimNode> {
        self.nodes.get_mut(idx)
    }

    /// Get all edges
    pub fn edges(&self) -> &[SimEdge] {
        &self.edges
    }

    /// Get all nodes
    pub fn nodes(&self) -> &[SimNode] {
        &self.nodes
    }

    /// Find node index by ID
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Get node neighbors (connected by edges)
    pub fn neighbors(&self, idx: usize) -> Vec<usize> {
        let mut result = Vec::new();
        for edge in &self.edges {
            if edge.source_idx == idx {
                result.push(edge.target_idx);
            } else if edge.target_idx == idx {
                result.push(edge.source_idx);
            }
        }
        result
    }

    /// Run one simulation step
    pub fn step(&mut self) -> SimulationResult {
        let n = self.nodes.len();
        if n == 0 {
            return SimulationResult::Converged;
        }

        // Apply forces
        self.apply_repulsion();
        self.apply_attraction();
        self.apply_gravity();

        // Integrate (update positions)
        let mut total_energy = 0.0;
        for node in &mut self.nodes {
            total_energy += node.kinetic_energy();
            node.integrate(self.damping);
        }

        // Check convergence
        if total_energy < self.convergence_threshold && n > 1 {
            return SimulationResult::Converged;
        }

        SimulationResult::Running
    }

    /// Run the simulation to convergence (or max iterations)
    pub fn run(&mut self) -> SimulationResult {
        for i in 0..self.max_iterations {
            let result = self.step();
            if result.is_converged() {
                return result;
            }
            if i == self.max_iterations - 1 {
                return SimulationResult::MaxIterationsReached;
            }
        }
        SimulationResult::MaxIterationsReached
    }

    /// Apply repulsion forces between all node pairs (O(n²) — acceptable for <500 nodes)
    fn apply_repulsion(&mut self) {
        let n = self.nodes.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.nodes[j].x - self.nodes[i].x;
                let dy = self.nodes[j].y - self.nodes[i].y;
                let dist_sq = dx * dx + dy * dy;
                let dist = dist_sq.sqrt().max(1.0);

                // Coulomb: F = k / d²
                let force = self.repulsion / dist_sq;

                // Normalize and scale
                let fx = (dx / dist) * force;
                let fy = (dy / dist) * force;

                // Apply equal and opposite forces
                if !self.nodes[i].pinned {
                    self.nodes[i].vx -= fx;
                    self.nodes[i].vy -= fy;
                }
                if !self.nodes[j].pinned {
                    self.nodes[j].vx += fx;
                    self.nodes[j].vy += fy;
                }
            }
        }
    }

    /// Apply attraction forces along edges (Hooke: F = -k · (d - rest))
    fn apply_attraction(&mut self) {
        for edge in &self.edges {
            let (Some(src), Some(tgt)) = (
                self.nodes.get(edge.source_idx),
                self.nodes.get(edge.target_idx),
            ) else {
                continue;
            };

            let dx = tgt.x - src.x;
            let dy = tgt.y - src.y;
            let dist = (dx * dx + dy * dy).sqrt().max(0.1);

            // Hooke's law: F = -k · (d - rest)
            let displacement = dist - self.rest_length;
            let force = self.attraction * displacement;

            let fx = (dx / dist) * force;
            let fy = (dy / dist) * force;

            if let Some(src_mut) = self.nodes.get_mut(edge.source_idx) {
                if !src_mut.pinned {
                    src_mut.vx += fx;
                    src_mut.vy += fy;
                }
            }
            if let Some(tgt_mut) = self.nodes.get_mut(edge.target_idx) {
                if !tgt_mut.pinned {
                    tgt_mut.vx -= fx;
                    tgt_mut.vy -= fy;
                }
            }
        }
    }

    /// Apply gravity toward center (prevents drift)
    fn apply_gravity(&mut self) {
        for node in &mut self.nodes {
            if !node.pinned {
                node.vx -= node.x * self.gravity;
                node.vy -= node.y * self.gravity;
            }
        }
    }

    /// Check if the simulation has converged
    pub fn is_converged(&self) -> bool {
        let total: f64 = self.nodes.iter().map(|n| n.kinetic_energy()).sum();
        total < self.convergence_threshold
    }

    /// Pin a node by index
    pub fn pin_node(&mut self, idx: usize) {
        if let Some(n) = self.nodes.get_mut(idx) {
            n.pin();
        }
    }

    /// Unpin a node by index
    pub fn unpin_node(&mut self, idx: usize) {
        if let Some(n) = self.nodes.get_mut(idx) {
            n.unpin();
        }
    }

    /// Move a pinned or free node to a new position
    pub fn move_node(&mut self, idx: usize, x: f64, y: f64) {
        if let Some(n) = self.nodes.get_mut(idx) {
            n.set_position(x, y);
            n.vx = 0.0;
            n.vy = 0.0;
        }
    }

    /// Force recompute of node radii (call after changing connection counts)
    pub fn recompute_radii(&mut self) {
        for node in &mut self.nodes {
            node.radius = SimNode::compute_radius(node.connection_count);
        }
    }

    /// Update connection count and recompute radius for a node
    pub fn update_connection_count(&mut self, idx: usize, count: usize) {
        if let Some(n) = self.nodes.get_mut(idx) {
            n.connection_count = count;
            n.radius = SimNode::compute_radius(count);
        }
    }

    /// Reset all nodes to random positions (useful for re-layout)
    pub fn randomize_positions(&mut self) {
        for node in &mut self.nodes {
            let angle = rand_angle();
            let r = rand_in_range(50.0, 150.0);
            node.x = angle.cos() * r;
            node.y = angle.sin() * r;
            node.vx = 0.0;
            node.vy = 0.0;
        }
    }

    /// Center all nodes around origin
    pub fn center(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let sum_x: f64 = self.nodes.iter().map(|n| n.x).sum();
        let sum_y: f64 = self.nodes.iter().map(|n| n.y).sum();
        let cx = sum_x / self.nodes.len() as f64;
        let cy = sum_y / self.nodes.len() as f64;
        for node in &mut self.nodes {
            node.x -= cx;
            node.y -= cy;
        }
    }
}

/// Result of a simulation step or run
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationResult {
    /// Simulation is still running
    Running,
    /// Simulation has converged (energy below threshold)
    Converged,
    /// Reached maximum iterations
    MaxIterationsReached,
}

impl SimulationResult {
    pub fn is_converged(&self) -> bool {
        matches!(self, Self::Converged)
    }
}

// Simple RNG helpers (no external dependency needed)
fn rand_angle() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Native: seed from subsecond timing
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        (nanos as f64 * 0.618033988749895) % (2.0 * std::f64::consts::PI)
    }
    #[cfg(target_arch = "wasm32")]
    {
        // WASM: deterministic pseudo-random via thread-local counter
        // SystemTime::now() is not available in WASM, so we use a simple LCG.
        // This doesn't need cryptographic strength — only initial scattering.
        use std::cell::Cell;
        thread_local! {
            static COUNTER: Cell<u64> = Cell::new(0);
        }
        COUNTER.with(|c| {
            let count = c.get();
            c.set(count.wrapping_add(1));
            let seed = count.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed as f64) / (u64::MAX as f64)) * std::f64::consts::TAU
        })
    }
}

fn rand_in_range(min: f64, max: f64) -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let t = (nanos as f64 * 0.618033988749895) % 1.0;
        min + t * (max - min)
    }
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::Cell;
        thread_local! {
            static COUNTER: Cell<u64> = Cell::new(0);
        }
        COUNTER.with(|c| {
            let count = c.get();
            c.set(count.wrapping_add(1));
            let seed = count.wrapping_mul(6364136223846793005).wrapping_add(1);
            let t = (seed as f64) / (u64::MAX as f64);
            min + t * (max - min)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_creation() {
        let sim = ForceSimulation::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec![
                "Node A".to_string(),
                "Node B".to_string(),
                "Node C".to_string(),
            ],
            vec![false, false, true],
            vec![0, 1],
            vec![1, 2],
        );
        assert_eq!(sim.node_count(), 3);
        assert_eq!(sim.edge_count(), 2);
    }

    #[test]
    fn test_neighbors() {
        let sim = ForceSimulation::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            vec![false, false, false],
            vec![0, 1],
            vec![1, 2],
        );
        // Node 0 is connected to 1
        let neighbors_0 = sim.neighbors(0);
        assert!(neighbors_0.contains(&1));
        // Node 1 is connected to 0 and 2
        let neighbors_1 = sim.neighbors(1);
        assert!(neighbors_1.contains(&0));
        assert!(neighbors_1.contains(&2));
    }

    #[test]
    fn test_simulation_convergence() {
        // Two connected nodes should converge
        let mut sim = ForceSimulation::new(
            vec!["a".to_string(), "b".to_string()],
            vec!["A".to_string(), "B".to_string()],
            vec![false, false],
            vec![0],
            vec![1],
        );
        let result = sim.run();
        assert!(result.is_converged());
    }

    #[test]
    fn test_node_radius() {
        // Node with 0 connections should have min radius
        let n0 = SimNode::new("0".to_string(), "Zero".to_string(), false, 0);
        assert_eq!(n0.radius, 8.0);

        // Node with many connections should have larger radius
        let n100 = SimNode::new("100".to_string(), "Many".to_string(), false, 100);
        assert!(n100.radius > 8.0);
        assert!(n100.radius <= 30.0);
    }

    #[test]
    fn test_pin_unpin() {
        let mut sim = ForceSimulation::new(
            vec!["a".to_string(), "b".to_string()],
            vec!["A".to_string(), "B".to_string()],
            vec![false, false],
            vec![0],
            vec![1],
        );
        sim.pin_node(0);
        let result = sim.run();
        // Should converge faster with pinned node
        assert!(result.is_converged());
    }

    #[test]
    fn test_with_params() {
        let params = SimulationParams {
            repulsion: Some(1000.0),
            attraction: Some(0.1),
            damping: Some(0.9),
            ..Default::default()
        };
        let mut sim = ForceSimulation::with_params(
            vec!["a".to_string(), "b".to_string()],
            vec!["A".to_string(), "B".to_string()],
            vec![false, false],
            vec![0],
            vec![1],
            params,
        );
        let result = sim.run();
        assert!(result.is_converged());
    }

    #[test]
    fn test_empty_simulation() {
        let mut sim = ForceSimulation::new(vec![], vec![], vec![], vec![], vec![]);
        let result = sim.run();
        assert!(result.is_converged());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut sim = ForceSimulation::new(
            vec!["a".to_string(), "b".to_string()],
            vec!["A".to_string(), "B".to_string()],
            vec![false, false],
            vec![0],
            vec![1],
        );
        sim.run();

        let nodes_json = serde_json::to_string(&sim.nodes()).unwrap();
        let nodes: Vec<SimNode> = serde_json::from_str(&nodes_json).unwrap();
        assert_eq!(nodes.len(), 2);

        let edges_json = serde_json::to_string(&sim.edges()).unwrap();
        let edges: Vec<SimEdge> = serde_json::from_str(&edges_json).unwrap();
        assert_eq!(edges.len(), 1);

        let result_json = serde_json::to_string(&sim.is_converged()).unwrap();
        assert_eq!(result_json, "true");
    }
}
