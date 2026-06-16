//! PropertyValue value object - typed property values

use std::collections::HashMap;
use std::fmt;

use url::Url;

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
    /// URL value
    Url(Url),
    /// Naive date value (YYYY-MM-DD, no timezone)
    NaiveDate(chrono::NaiveDate),
}

impl PropertyValue {
    /// Create a string property value
    pub fn string(s: impl Into<String>) -> Self {
        PropertyValue::String(s.into())
    }

    /// Create a text property value (alias for [`string()`]).
    ///
    /// Used in the canonicalization pipeline to align with spec vocabulary.
    #[must_use]
    pub fn text(s: impl Into<String>) -> Self {
        Self::string(s)
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

    /// Create a URL property value
    pub fn url(u: Url) -> Self {
        PropertyValue::Url(u)
    }

    /// Create a naive date property value (YYYY-MM-DD, no timezone).
    ///
    /// Use this for calendar dates that have no time or timezone component.
    pub fn naive_date(d: chrono::NaiveDate) -> Self {
        PropertyValue::NaiveDate(d)
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
            PropertyValue::Url(_) => "url",
            PropertyValue::NaiveDate(_) => "date", // Shared with Date (distinguished via serde shape)
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
            PropertyValue::Url(u) => serde_json::Value::String(u.to_string()),
            PropertyValue::NaiveDate(d) => {
                serde_json::Value::String(d.format("%Y-%m-%d").to_string())
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

    /// Returns a `(display_string, type_hint)` tuple suitable for
    /// serialisation. The type hint uses the same naming as
    /// `type_name()` so downstream type resolution stays consistent.
    ///
    /// Moved from `quilt-application/src/use_cases/template.rs`,
    /// where it previously lived as a free function. Now callers
    /// everywhere can access it through the entity.
    /// (quilt-architecture-review candidate #6)
    pub fn to_display_string(&self) -> (String, String) {
        let stringified = match self {
            PropertyValue::String(s) => s.clone(),
            PropertyValue::Boolean(b) => b.to_string(),
            PropertyValue::Integer(i) => i.to_string(),
            PropertyValue::Float(f) => f.to_string(),
            PropertyValue::Date(d) => d.to_rfc3339(),
            PropertyValue::Ref(s) => s.clone(),
            PropertyValue::Array(arr) => {
                let parts: Vec<String> = arr
                    .iter()
                    .map(|v| v.to_display_string())
                    .map(|(s, _)| s)
                    .collect();
                format!("[{}]", parts.join(", "))
            }
            PropertyValue::Url(u) => u.to_string(),
            PropertyValue::NaiveDate(d) => d.format("%Y-%m-%d").to_string(),
        };
        let type_hint = self.type_name().to_string();
        (stringified, type_hint)
    }

    /// Convenience: returns only the display string without the type hint.
    pub fn as_display_string(&self) -> String {
        self.to_display_string().0
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
            PropertyValue::Url(u) => write!(f, "{}", u),
            PropertyValue::NaiveDate(d) => write!(f, "{}", d.format("%Y-%m-%d")),
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
    use chrono::TimeZone;

    // ── Constructors ──────────────────────────────────────────────

    #[test]
    fn test_constructors() {
        assert_eq!(
            PropertyValue::string("hello"),
            PropertyValue::String("hello".into())
        );
        assert_eq!(PropertyValue::boolean(true), PropertyValue::Boolean(true));
        assert_eq!(PropertyValue::integer(42), PropertyValue::Integer(42));
        assert_eq!(PropertyValue::float(3.14), PropertyValue::Float(3.14));
        assert_eq!(
            PropertyValue::reference("mypage"),
            PropertyValue::Ref("mypage".into())
        );
    }

    #[test]
    fn test_string_constructor_accepts_string_types() {
        let s = PropertyValue::string("owned");
        assert_eq!(s, PropertyValue::String("owned".into()));

        let s = PropertyValue::string(String::from("owned"));
        assert_eq!(s, PropertyValue::String("owned".into()));
    }

    // ── text alias ────────────────────────────────────────────────

    #[test]
    fn test_text_constructor_matches_string_constructor() {
        assert_eq!(PropertyValue::text("hello"), PropertyValue::string("hello"));
        assert_eq!(
            PropertyValue::text(String::from("world")),
            PropertyValue::string(String::from("world"))
        );
    }

    #[test]
    fn test_text_accepts_str_and_string() {
        let s1 = PropertyValue::text("owned");
        assert_eq!(s1, PropertyValue::String("owned".into()));

        let s2 = PropertyValue::text(String::from("owned"));
        assert_eq!(s2, PropertyValue::String("owned".into()));
    }

    // ── type_name ─────────────────────────────────────────────────

    #[test]
    fn test_type_name_all_variants() {
        assert_eq!(PropertyValue::String("x".into()).type_name(), "string");
        assert_eq!(PropertyValue::Boolean(true).type_name(), "boolean");
        assert_eq!(PropertyValue::Integer(1).type_name(), "integer");
        assert_eq!(PropertyValue::Float(1.0).type_name(), "float");
        assert_eq!(
            PropertyValue::Date(chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
                .type_name(),
            "date"
        );
        assert_eq!(PropertyValue::Ref("p".into()).type_name(), "ref");
        assert_eq!(PropertyValue::Array(vec![]).type_name(), "array");
    }

    // ── to_json / from_json roundtrip ─────────────────────────────

    #[test]
    fn test_json_roundtrip_string() {
        let original = PropertyValue::String("hello".into());
        let json = original.to_json();
        assert_eq!(json, serde_json::json!("hello"));
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_json_roundtrip_boolean() {
        let original = PropertyValue::Boolean(true);
        let json = original.to_json();
        assert_eq!(json, serde_json::json!(true));
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_json_roundtrip_integer() {
        let original = PropertyValue::Integer(42);
        let json = original.to_json();
        assert_eq!(json, serde_json::json!(42));
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_json_roundtrip_integer_negative() {
        let original = PropertyValue::Integer(-7);
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_json_roundtrip_float() {
        let original = PropertyValue::Float(3.14);
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        match restored {
            PropertyValue::Float(f) => assert!((f - 3.14).abs() < f64::EPSILON),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn test_json_roundtrip_date() {
        let dt = chrono::Utc
            .with_ymd_and_hms(2026, 5, 15, 10, 30, 0)
            .unwrap();
        let original = PropertyValue::Date(dt);
        let json = original.to_json();
        assert!(json.as_str().unwrap().starts_with("2026-05-15"));
        // from_json cannot reconstruct a Date from a JSON string in the current impl
        // (it becomes a String). Document this behavior.
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(restored, PropertyValue::String(dt.to_rfc3339()));
    }

    #[test]
    fn test_json_roundtrip_ref() {
        let original = PropertyValue::Ref("mypage".into());
        let json = original.to_json();
        assert_eq!(json, serde_json::json!("mypage"));
        // from_json interprets as String (no way to distinguish from plain string)
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(restored, PropertyValue::String("mypage".into()));
    }

    #[test]
    fn test_json_roundtrip_array() {
        let original = PropertyValue::Array(vec![
            PropertyValue::String("a".into()),
            PropertyValue::Integer(1),
        ]);
        let json = original.to_json();
        assert_eq!(json, serde_json::json!(["a", 1]));
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_json_roundtrip_nested_array() {
        let original = PropertyValue::Array(vec![
            PropertyValue::Array(vec![PropertyValue::Integer(1), PropertyValue::Integer(2)]),
            PropertyValue::String("outer".into()),
        ]);
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_json_roundtrip_empty_array() {
        let original = PropertyValue::Array(vec![]);
        let json = original.to_json();
        assert_eq!(json, serde_json::json!([]));
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_from_json_null_returns_none() {
        assert_eq!(PropertyValue::from_json(&serde_json::Value::Null), None);
    }

    #[test]
    fn test_from_json_object_returns_none() {
        let obj = serde_json::json!({"key": "value"});
        assert_eq!(PropertyValue::from_json(&obj), None);
    }

    // ── Display ───────────────────────────────────────────────────

    #[test]
    fn test_display_string() {
        assert_eq!(
            format!("{}", PropertyValue::String("hello".into())),
            "hello"
        );
    }

    #[test]
    fn test_display_boolean() {
        assert_eq!(format!("{}", PropertyValue::Boolean(true)), "true");
        assert_eq!(format!("{}", PropertyValue::Boolean(false)), "false");
    }

    #[test]
    fn test_display_integer() {
        assert_eq!(format!("{}", PropertyValue::Integer(42)), "42");
        assert_eq!(format!("{}", PropertyValue::Integer(-5)), "-5");
    }

    #[test]
    fn test_display_float() {
        let val = PropertyValue::Float(3.14);
        let s = format!("{}", val);
        assert!(s.starts_with("3.14"));
    }

    #[test]
    fn test_display_date() {
        let dt = chrono::Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
        let val = PropertyValue::Date(dt);
        assert_eq!(format!("{}", val), "2026-06-02");
    }

    #[test]
    fn test_display_ref() {
        let val = PropertyValue::Ref("mypage".into());
        assert_eq!(format!("{}", val), "[[mypage]]");
    }

    #[test]
    fn test_display_array() {
        let val = PropertyValue::Array(vec![
            PropertyValue::String("a".into()),
            PropertyValue::Integer(1),
        ]);
        assert_eq!(format!("{}", val), "[a, 1]");
    }

    #[test]
    fn test_display_empty_array() {
        let val = PropertyValue::Array(vec![]);
        assert_eq!(format!("{}", val), "[]");
    }

    // ── Default ───────────────────────────────────────────────────

    #[test]
    fn test_default_is_empty_string() {
        assert_eq!(
            PropertyValue::default(),
            PropertyValue::String(String::new())
        );
    }

    // ── parse_properties (exercises normalize_property_name internally) ──

    #[test]
    fn test_parse_properties_normalizes_keys() {
        let mut map = serde_json::Map::new();
        // Keys with mixed case, slashes, spaces, underscores — all get normalized
        map.insert(
            "My Title".to_string(),
            serde_json::Value::String("hello".to_string()),
        );
        map.insert(
            "FOO/BAR".to_string(),
            serde_json::Value::String("baz".to_string()),
        );
        map.insert(
            "snake_case".to_string(),
            serde_json::Value::String("val".to_string()),
        );
        let props = parse_properties(&map);

        assert_eq!(
            props.get("my-title"),
            Some(&PropertyValue::String("hello".into()))
        );
        assert_eq!(
            props.get("foo-bar"),
            Some(&PropertyValue::String("baz".into()))
        );
        assert_eq!(
            props.get("snake-case"),
            Some(&PropertyValue::String("val".into()))
        );
    }

    #[test]
    fn test_parse_properties_single() {
        let mut map = serde_json::Map::new();
        map.insert(
            "status".to_string(),
            serde_json::Value::String("draft".to_string()),
        );
        let props = parse_properties(&map);
        assert_eq!(props.len(), 1);
        assert_eq!(
            props.get("status"),
            Some(&PropertyValue::String("draft".into()))
        );
    }

    #[test]
    fn test_parse_properties_multiple() {
        let mut map = serde_json::Map::new();
        map.insert("count".to_string(), serde_json::json!(5));
        map.insert("active".to_string(), serde_json::json!(true));
        let props = parse_properties(&map);
        assert_eq!(props.len(), 2);
        assert_eq!(props.get("count"), Some(&PropertyValue::Integer(5)));
        assert_eq!(props.get("active"), Some(&PropertyValue::Boolean(true)));
    }

    #[test]
    fn test_parse_properties_skips_invalid_values() {
        let mut map = serde_json::Map::new();
        map.insert(
            "valid".to_string(),
            serde_json::Value::String("ok".to_string()),
        );
        map.insert("invalid".to_string(), serde_json::Value::Null);
        let props = parse_properties(&map);
        assert_eq!(props.len(), 1);
        assert!(props.contains_key("valid"));
        assert!(!props.contains_key("invalid"));
    }

    #[test]
    fn test_parse_properties_empty() {
        let map = serde_json::Map::new();
        let props = parse_properties(&map);
        assert!(props.is_empty());
    }

    // ── Url variant ──────────────────────────────────────────────────────────────

    #[test]
    fn url_constructor_creates_url_variant() {
        let url = Url::parse("https://quilt.dev").unwrap();
        let pv = PropertyValue::url(url.clone());
        assert!(matches!(pv, PropertyValue::Url(u) if u == url));
    }

    #[test]
    fn type_name_for_url_is_url() {
        let url = Url::parse("https://x.com").unwrap();
        assert_eq!(PropertyValue::url(url).type_name(), "url");
    }

    #[test]
    fn to_json_for_url_emits_string() {
        let url = Url::parse("https://quilt.dev/path?q=1").unwrap();
        let pv = PropertyValue::url(url);
        assert_eq!(pv.to_json(), serde_json::json!("https://quilt.dev/path?q=1"));
    }

    #[test]
    fn to_json_for_url_roundtrips_through_from_json_as_string() {
        // from_json cannot distinguish a URL string from a plain string (flat JSON limitation)
        let url = Url::parse("https://quilt.dev").unwrap();
        let pv = PropertyValue::url(url);
        let json = pv.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert!(matches!(restored, PropertyValue::String(s) if s.starts_with("https://quilt.dev")));
    }

    #[test]
    fn display_for_url() {
        let url = Url::parse("https://quilt.dev").unwrap();
        let pv = PropertyValue::url(url);
        let display = format!("{}", pv);
        assert!(display.starts_with("https://quilt.dev"), "got: {}", display);
    }

    #[test]
    fn to_display_string_for_url() {
        let url = Url::parse("https://quilt.dev").unwrap();
        let pv = PropertyValue::url(url);
        let (s, t) = pv.to_display_string();
        assert!(s.starts_with("https://quilt.dev"), "got string: {}", s);
        assert_eq!(t, "url");
    }

    #[test]
    fn serde_roundtrip_for_url_uses_externally_tagged_shape() {
        let url = Url::parse("https://quilt.dev").unwrap();
        let pv = PropertyValue::url(url);
        let s = serde_json::to_string(&pv).unwrap();
        // Externally tagged shape: {"Url":"..."} where ... is the URL string
        assert!(s.starts_with(r#"{"Url":"https://quilt.dev"#), "got: {}", s);
        assert!(s.ends_with(r#""}"#), "got: {}", s);
    }

    #[test]
    fn url_inside_array_roundtrips() {
        let url = Url::parse("https://quilt.dev").unwrap();
        let arr = PropertyValue::Array(vec![PropertyValue::url(url)]);
        let json = arr.to_json();
        // to_json produces a JSON array with the URL as a string (lossy)
        // Note: url crate may add trailing slash
        let s = json.as_array().unwrap()[0].as_str().unwrap();
        assert!(s.starts_with("https://quilt.dev"));
    }

    // ── NaiveDate variant ───────────────────────────────────────────────────────

    #[test]
    fn naive_date_constructor_creates_naive_date_variant() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let pv = PropertyValue::naive_date(d);
        assert!(matches!(pv, PropertyValue::NaiveDate(d2) if d2 == d));
    }

    #[test]
    fn type_name_for_naive_date_is_date() {
        // NaiveDate shares "date" type_name with Date (distinguished via serde shape)
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        assert_eq!(PropertyValue::naive_date(d).type_name(), "date");
    }

    #[test]
    fn to_json_for_naive_date_emits_iso_date_string() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let pv = PropertyValue::naive_date(d);
        assert_eq!(pv.to_json(), serde_json::json!("2026-06-15"));
    }

    #[test]
    fn to_json_for_naive_date_roundtrips_through_from_json_as_string() {
        // from_json cannot distinguish a date string from a plain string (flat JSON limitation)
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let pv = PropertyValue::naive_date(d);
        let json = pv.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert!(matches!(restored, PropertyValue::String(s) if s == "2026-06-15"));
    }

    #[test]
    fn display_for_naive_date() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let pv = PropertyValue::naive_date(d);
        assert_eq!(format!("{}", pv), "2026-06-15");
    }

    #[test]
    fn to_display_string_for_naive_date() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let pv = PropertyValue::naive_date(d);
        assert_eq!(pv.to_display_string(), ("2026-06-15".to_string(), "date".to_string()));
    }

    #[test]
    fn serde_roundtrip_for_naive_date_uses_externally_tagged_shape() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let pv = PropertyValue::naive_date(d);
        let s = serde_json::to_string(&pv).unwrap();
        assert_eq!(s, r#"{"NaiveDate":"2026-06-15"}"#);
    }

    #[test]
    fn naive_date_and_date_distinguishable_via_serde() {
        // Date uses RFC3339 (with time+timezone), NaiveDate uses YYYY-MM-DD
        let dt = chrono::Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
        let date_pv = PropertyValue::Date(dt);
        let naive_pv = PropertyValue::naive_date(dt.date_naive());

        let date_json = serde_json::to_string(&date_pv).unwrap();
        let naive_json = serde_json::to_string(&naive_pv).unwrap();
        assert_ne!(date_json, naive_json, "Date and NaiveDate must be distinguishable via serde");
        assert!(
            date_json.contains("+00:00") || date_json.contains("T00:00:00"),
            "Date must use RFC3339 format: got {}",
            date_json
        );
        assert!(
            naive_json.contains("NaiveDate"),
            "NaiveDate must use externally tagged shape: got {}",
            naive_json
        );
    }

    #[test]
    fn naive_date_inside_array_roundtrips() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let arr = PropertyValue::Array(vec![PropertyValue::naive_date(d)]);
        let json = arr.to_json();
        // to_json produces a JSON array with the date as a string (lossy)
        assert_eq!(json, serde_json::json!(["2026-06-15"]));
    }
}
