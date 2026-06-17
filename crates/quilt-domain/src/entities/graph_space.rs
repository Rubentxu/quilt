//! GraphSpace entity - persisted graph-level metadata
//!
//! This is a singleton table that stores metadata about the graph itself,
//! such as the display name shown in the UI.

use crate::errors::DomainError;
use serde::Serialize;

/// Graph space metadata.
///
/// This is persisted in the database and contains graph-level information
/// like the display name shown in the graph selector.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GraphSpace {
    /// Display name for the graph (e.g., "Work Notes", "Personal Journal")
    pub name: String,

    /// Optional description of the graph
    pub description: String,

    /// Schema version for future migrations
    pub version: String,
}

impl Default for GraphSpace {
    fn default() -> Self {
        Self {
            name: "My Graph".to_string(),
            description: String::new(),
            version: "1.0".to_string(),
        }
    }
}

impl GraphSpace {
    /// Validate the graph space metadata.
    ///
    /// # Errors
    /// Returns error if name is empty.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.is_empty() {
            return Err(DomainError::InvalidConfiguration(
                "graph name cannot be empty".to_string(),
            ));
        }

        if self.name.len() > 255 {
            return Err(DomainError::InvalidConfiguration(
                "graph name cannot exceed 255 characters".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_graph_space() {
        let gs = GraphSpace::default();
        assert_eq!(gs.name, "My Graph");
        assert_eq!(gs.description, "");
        assert_eq!(gs.version, "1.0");
    }

    #[test]
    fn test_validate_valid_graph_space() {
        let gs = GraphSpace::default();
        assert!(gs.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let gs = GraphSpace {
            name: "".to_string(),
            ..Default::default()
        };
        let result = gs.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidConfiguration(msg) => {
                assert!(msg.contains("name cannot be empty"));
            }
            _ => panic!("Expected InvalidConfiguration error"),
        }
    }

    #[test]
    fn test_validate_name_too_long() {
        let gs = GraphSpace {
            name: "a".repeat(256),
            ..Default::default()
        };
        let result = gs.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidConfiguration(msg) => {
                assert!(msg.contains("name cannot exceed 255"));
            }
            _ => panic!("Expected InvalidConfiguration error"),
        }
    }
}
