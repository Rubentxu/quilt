//! Graph Utilities — Shared Helper Functions for Cognitive Engines
//!
//! This module provides common utilities used across multiple cognitive engines,
//! reducing duplication and ensuring consistent behavior.

use quilt_domain::value_objects::Uuid;

/// Compare two Uuids by their inner bytes (lexicographic ordering).
///
/// This is used for deterministic ordering when Uuids need to be sorted
/// or compared in a consistent way across engines.
pub fn uuid_lt(a: &Uuid, b: &Uuid) -> bool {
    a.as_bytes() < b.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_lt_ordering() {
        let uuid1 = Uuid::from_bytes([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let uuid2 = Uuid::from_bytes([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert!(uuid_lt(&uuid1, &uuid2));
        assert!(!uuid_lt(&uuid2, &uuid1));
    }

    #[test]
    fn test_uuid_lt_reflexive() {
        let uuid1 = Uuid::from_bytes([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!uuid_lt(&uuid1, &uuid1));
    }
}