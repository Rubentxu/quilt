//! PropertyValue value object - typed property values

use std::collections::HashMap;
use std::fmt;

/// PropertyValue represents a typed property value.
///
/// Quilt properties can have different types:
/// - String: plain text
/// - Boolean: true/false
/// - Integer: whole numbers
/// - Float: decimal numbers
/// - Date: timestamps
/// - Reference: links to other entities
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PropertyValue {
    /// Plain text value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Integer number
    Integer(i64),
    /// Floating point number
    Float(f64),
    /// Date/time value
    Date(chrono::DateTime<chrono::Utc>),
    /// Reference to another entity (page or block)
    Ref(String),
    /// Array of values
    Array(Vec<PropertyValue>),
}

impl PropertyValue {
    /// Create a string property value
    pub fn string(s: impl Into<String>) -> Self {
        PropertyValue::String(s.into())
    }

    /// Create a boolean property value
    pub fn boolean(b: bool) -> Self {
        PropertyValue::Boolean(b)
    }

    /// Create an integer property value
    pub fn integer(i: i64) -> Self {
        PropertyValue::Integer(i)
    }

    /// Create a float property value
    pub fn float(f: f64) -> Self {
        PropertyValue::Float(f)
    }

    /// Create a date property value
    pub fn date(dt: chrono::DateTime<chrono::Utc>) -> Self {
        PropertyValue::Date(dt)
    }

    /// Create a reference property value
    pub fn reference(s: impl Into<String>) -> Self {
        PropertyValue::Ref(s.into())
    }

    /// Get the type name
    pub fn type_name(&self) -> &'static str {
        match self {
            PropertyValue::String(_) => "string",
            PropertyValue::Boolean(_) => "boolean",
            PropertyValue::Integer(_) => "integer",
            PropertyValue::Float(_) => "float",
            PropertyValue::Date(_) => "date",
            PropertyValue::Ref(_) => "ref",
            PropertyValue::Array(_) => "array",
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            PropertyValue::String(s) => serde_json::Value::String(s.clone()),
            PropertyValue::Boolean(b) => serde_json::Value::Bool(*b),
            PropertyValue::Integer(i) => serde_json::Value::Number((*i).into()),
            PropertyValue::Float(f) => {
                serde_json::from_value(serde_json::json!(*f)).unwrap_or(serde_json::Value::Null)
            }
            PropertyValue::Date(dt) => serde_json::Value::String(dt.to_rfc3339()),
            PropertyValue::Ref(s) => serde_json::Value::String(s.clone()),
            PropertyValue::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| v.to_json()).collect())
            }
        }
    }

    /// Parse from JSON value
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::String(s) => Some(PropertyValue::String(s.clone())),
            serde_json::Value::Bool(b) => Some(PropertyValue::Boolean(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(PropertyValue::Integer(i))
                } else {
                    n.as_f64().map(PropertyValue::Float)
                }
            }
            serde_json::Value::Array(arr) => Some(PropertyValue::Array(
                arr.iter().filter_map(Self::from_json).collect(),
            )),
            _ => None,
        }
    }
}

impl fmt::Display for PropertyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyValue::String(s) => write!(f, "{}", s),
            PropertyValue::Boolean(b) => write!(f, "{}", b),
            PropertyValue::Integer(i) => write!(f, "{}", i),
            PropertyValue::Float(fl) => write!(f, "{}", fl),
            PropertyValue::Date(dt) => write!(f, "{}", dt.format("%Y-%m-%d")),
            PropertyValue::Ref(r) => write!(f, "[[{}]]", r),
            PropertyValue::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl Default for PropertyValue {
    fn default() -> Self {
        PropertyValue::String(String::new())
    }
}

/// Normalize a property name according to Quilt rules:
/// - Convert to lowercase
/// - Replace `/` with `-`
/// - Replace spaces with `-`
/// - Replace `_` with `-`
#[allow(dead_code)]
pub fn normalize_property_name(name: &str) -> String {
    name.to_lowercase().replace(['/', ' ', '_'], "-")
}

/// Parse properties from a JSON object
#[allow(dead_code)]
pub fn parse_properties(
    json: &serde_json::Map<String, serde_json::Value>,
) -> HashMap<String, PropertyValue> {
    json.iter()
        .filter_map(|(k, v)| {
            let normalized_key = normalize_property_name(k);
            let value = PropertyValue::from_json(v)?;
            Some((normalized_key, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_types() {
        let s = PropertyValue::string("hello");
        assert_eq!(s.type_name(), "string");

        let b = PropertyValue::boolean(true);
        assert_eq!(b.type_name(), "boolean");

        let i = PropertyValue::integer(42);
        assert_eq!(i.type_name(), "integer");
    }

    #[test]
    fn test_normalize_property_name() {
        assert_eq!(normalize_property_name("Title"), "title");
        assert_eq!(normalize_property_name("foo/bar"), "foo-bar");
        assert_eq!(normalize_property_name("foo bar"), "foo-bar");
        assert_eq!(normalize_property_name("foo_bar"), "foo-bar");
    }

    #[test]
    fn test_json_conversion() {
        let original = PropertyValue::string("test");
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }
}
