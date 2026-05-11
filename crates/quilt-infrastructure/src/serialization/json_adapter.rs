//! JSON serialization adapter

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct JsonSerializer;

impl JsonSerializer {
    /// Serialize a value to bytes
    pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(value)?;
        Ok(json)
    }

    /// Deserialize bytes to a value
    pub fn deserialize<'a, T: Deserialize<'a>>(data: &'a [u8]) -> Result<T> {
        let val = serde_json::from_slice(data)?;
        Ok(val)
    }
}
