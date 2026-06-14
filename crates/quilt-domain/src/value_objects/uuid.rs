//! Uuid value object - wrapper around UUID with convenience methods

use crate::errors::DomainError;
use std::fmt;
use std::str::FromStr;

/// Uuid wraps a UUID with convenience methods for Quilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Uuid(uuid::Uuid);

impl Uuid {
    /// Create a new random UUID
    pub fn new_v4() -> Self {
        Uuid(uuid::Uuid::new_v4())
    }

    /// Create a UUID from raw bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Uuid(uuid::Uuid::from_bytes(bytes))
    }

    /// Create a UUID from a string (any valid format)
    pub fn parse_str(s: &str) -> Result<Self, DomainError> {
        uuid::Uuid::parse_str(s)
            .map(Uuid)
            .map_err(|e| DomainError::ParseError(format!("Invalid UUID: {} - {}", s, e)))
    }

    /// Get the underlying UUID as bytes
    pub fn as_bytes(&self) -> [u8; 16] {
        *self.0.as_bytes()
    }

    /// Get a short representation (first 8 characters)
    pub fn short(&self) -> String {
        self.0.to_string()[..8].to_string()
    }

    /// Nil UUID (all zeros)
    pub fn nil() -> Self {
        Uuid(uuid::Uuid::nil())
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Uuid {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::from_str(s).map(Uuid)
    }
}

impl From<uuid::Uuid> for Uuid {
    fn from(uuid: uuid::Uuid) -> Self {
        Uuid(uuid)
    }
}

impl From<Uuid> for uuid::Uuid {
    fn from(uuid: Uuid) -> Self {
        uuid.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_uuid() {
        let uuid = Uuid::new_v4();
        assert_eq!(uuid.to_string().len(), 36);
    }

    #[test]
    fn test_parse_uuid() {
        let original = Uuid::new_v4();
        let parsed = Uuid::parse_str(&original.to_string()).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_short() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(uuid.short(), "550e8400");
    }

    #[test]
    fn test_nil() {
        let nil = Uuid::nil();
        assert_eq!(nil.to_string(), "00000000-0000-0000-0000-000000000000");
    }
}
