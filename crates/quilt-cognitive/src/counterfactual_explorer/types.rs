//! Counterfactual Explorer types
//!
//! Defines types for exploring counterfactual scenarios and their consequences.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A branch in a counterfactual exploration tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualBranch {
    /// Human-readable label for this branch.
    pub label: String,
    /// Consequences of following this branch.
    pub consequences: Vec<String>,
    /// Assumptions that are challenged by this branch.
    pub assumptions_challenged: Vec<String>,
    /// Confidence in this branch exploration (0.0-1.0).
    pub confidence: f32,
}

/// A tree of counterfactual branches exploring alternative scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualTree {
    /// All branches explored.
    pub branches: Vec<CounterfactualBranch>,
    /// The topic or scenario being explored.
    pub topic: String,
    /// When this tree was generated.
    pub generated_at: DateTime<Utc>,
}

impl CounterfactualTree {
    /// Create a new counterfactual tree.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            branches: Vec::new(),
            topic: topic.into(),
            generated_at: Utc::now(),
        }
    }

    /// Add a branch to the tree.
    pub fn add_branch(&mut self, branch: CounterfactualBranch) {
        self.branches.push(branch);
    }
}

impl CounterfactualBranch {
    /// Create a new counterfactual branch.
    pub fn new(
        label: impl Into<String>,
        consequences: Vec<String>,
        assumptions_challenged: Vec<String>,
        confidence: f32,
    ) -> Self {
        Self {
            label: label.into(),
            consequences,
            assumptions_challenged,
            confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counterfactual_tree_new() {
        let tree = CounterfactualTree::new("Should I learn Rust?");
        assert_eq!(tree.topic, "Should I learn Rust?");
        assert!(tree.branches.is_empty());
    }

    #[test]
    fn test_counterfactual_tree_add_branch() {
        let mut tree = CounterfactualTree::new("Test scenario");
        let branch = CounterfactualBranch::new(
            "If I choose A",
            vec!["Consequence 1".to_string()],
            vec!["Assumption X".to_string()],
            0.8,
        );
        tree.add_branch(branch);
        assert_eq!(tree.branches.len(), 1);
    }

    #[test]
    fn test_counterfactual_branch_new() {
        let branch = CounterfactualBranch::new(
            "Alternative path",
            vec!["Good outcome".to_string(), "Bad outcome".to_string()],
            vec!["Old assumption".to_string()],
            0.75,
        );
        assert_eq!(branch.label, "Alternative path");
        assert_eq!(branch.consequences.len(), 2);
        assert_eq!(branch.assumptions_challenged.len(), 1);
        assert_eq!(branch.confidence, 0.75);
    }

    #[test]
    fn test_confidence_range() {
        let branch = CounterfactualBranch::new("Test", vec![], vec![], 0.5);
        assert!(branch.confidence >= 0.0 && branch.confidence <= 1.0);
    }
}
