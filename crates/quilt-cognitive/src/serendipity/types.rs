//! Types for Serendipity Engine

use quilt_domain::value_objects::Uuid as DomainUuid;
use serde::{Deserialize, Serialize};

/// A discovered serendipitous connection between two blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerendipityConnection {
    pub idea_a: DomainUuid,
    pub idea_b: DomainUuid,
    pub bridge_concept: Option<String>,
    pub confidence: f32,
    pub explanation: String,
    pub connection_type: ConnectionType,
}

impl Default for SerendipityConnection {
    fn default() -> Self {
        Self {
            idea_a: DomainUuid::nil(),
            idea_b: DomainUuid::nil(),
            bridge_concept: None,
            confidence: 0.0,
            explanation: String::new(),
            connection_type: ConnectionType::Structural,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    #[default]
    Structural,
    Content,
    Temporal,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerendipityQuery {
    pub topic: Option<String>,
    pub limit: usize,
    pub offset: usize,
    pub min_confidence: f32,
    pub temporal_window_days: Option<i64>,
    /// Page to search for connections within. Required for `temporal_window_days: None`.
    pub page_id: Option<DomainUuid>,
}

impl Default for SerendipityQuery {
    fn default() -> Self {
        Self {
            topic: None,
            limit: 20,
            offset: 0,
            min_confidence: 0.3,
            temporal_window_days: Some(30),
            page_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SerendipityOptions {
    pub halflife_days: f64,
    pub min_bridge_threshold: f32,
    pub cache_ttl_secs: u64,
    pub default_min_confidence: f32,
}

impl Default for SerendipityOptions {
    fn default() -> Self {
        Self {
            halflife_days: 7.0,
            min_bridge_threshold: 0.4,
            cache_ttl_secs: 300,
            default_min_confidence: 0.3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serendipity_connection_default() {
        let conn = SerendipityConnection::default();
        assert_eq!(conn.idea_a, DomainUuid::nil());
        assert_eq!(conn.idea_b, DomainUuid::nil());
        assert!(conn.bridge_concept.is_none());
        assert_eq!(conn.confidence, 0.0);
    }

    #[test]
    fn test_connection_type_default() {
        assert_eq!(ConnectionType::default(), ConnectionType::Structural);
    }

    #[test]
    fn test_serendipity_query_default() {
        let q = SerendipityQuery::default();
        assert_eq!(q.limit, 20);
        assert_eq!(q.offset, 0);
        assert_eq!(q.min_confidence, 0.3);
    }

    #[test]
    fn test_serendipity_options_default() {
        let opts = SerendipityOptions::default();
        assert_eq!(opts.halflife_days, 7.0);
        assert_eq!(opts.min_bridge_threshold, 0.4);
        assert_eq!(opts.cache_ttl_secs, 300);
    }
}
