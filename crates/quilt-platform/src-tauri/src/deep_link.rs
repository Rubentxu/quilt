//! Deep link handling for quilt:// URL scheme
//!
//! This module parses quilt:// URLs into typed DeepLinkTarget values
//! that can be used for navigation.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Deep link target parsed from quilt:// URL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DeepLinkTarget {
    /// Navigate to a specific page
    Page {
        graph_id: Option<String>,
        page_name: String,
    },
    /// Navigate to a specific block on a page
    Block {
        graph_id: Option<String>,
        page_name: String,
        block_uuid: Uuid,
    },
}

/// Error type for deep link parsing
#[derive(Debug, Clone)]
pub struct DeepLinkError(String);

impl std::fmt::Display for DeepLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DeepLinkError {}

impl From<DeepLinkError> for String {
    fn from(e: DeepLinkError) -> Self {
        e.0
    }
}

/// Parser for quilt:// deep links
///
/// Supports the following URL formats:
/// - `quilt://page/{name}` - Navigate to page in current graph
/// - `quilt://page/{name}#block-{uuid}` - Navigate to block on page
/// - `quilt://graph/{id}/page/{name}` - Navigate to page in specific graph
/// - `quilt://graph/{id}/page/{name}#block-{uuid}` - Navigate to block in specific graph
pub struct DeepLinkParser;

impl DeepLinkParser {
    /// Parse a quilt:// URL into a DeepLinkTarget
    ///
    /// # Arguments
    /// * `url` - The URL to parse (e.g., "quilt://page/Home" or "quilt://graph/my-graph/page/Notes#block-abc123")
    ///
    /// # Examples
    /// ```
    /// use quilt_tauri::deep_link::{DeepLinkParser, DeepLinkTarget};
    ///
    /// let target = DeepLinkParser::parse("quilt://page/Home").unwrap();
    /// assert!(matches!(target, DeepLinkTarget::Page { page_name, .. } if page_name == "Home"));
    /// ```
    pub fn parse(url: &str) -> Result<DeepLinkTarget, String> {
        // Parse the URL
        let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

        // Check scheme
        if parsed.scheme() != "quilt" {
            return Err(format!("Unsupported URL scheme: {}", parsed.scheme()));
        }

        // Get the host (indicates URL type) and path
        let host = parsed.host_str().unwrap_or("");
        let path = parsed.path();

        // Parse the path segments
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // Handle fragment (block reference) - extract UUID if present
        let block_uuid: Option<Uuid> = if let Some(fragment) = parsed.fragment() {
            if let Some(uuid_str) = fragment.strip_prefix("block-") {
                Some(
                    Uuid::parse_str(uuid_str)
                        .map_err(|_| format!("Invalid block UUID: {}", uuid_str))?,
                )
            } else {
                return Err(format!("Unrecognized URL fragment: {}", fragment));
            }
        } else {
            None
        };

        // Route based on host
        match host {
            "page" => Self::parse_page_url(&segments, block_uuid),
            "graph" => Self::parse_graph_url(&segments, block_uuid),
            _ => Err(format!(
                "Unrecognized URL path: /{}{}",
                host,
                path.trim_start_matches('/')
            )),
        }
    }

    /// Parse a quilt://page/{name} URL (host is "page", segments = [page_name])
    fn parse_page_url(
        segments: &[&str],
        fragment_uuid: Option<Uuid>,
    ) -> Result<DeepLinkTarget, String> {
        // segments = [page_name] when host is "page"
        if segments.is_empty() {
            return Err("Invalid URL: missing page name".to_string());
        }

        let page_name = Self::decode_percent(segments[0])?;

        if let Some(uuid) = fragment_uuid {
            Ok(DeepLinkTarget::Block {
                graph_id: None,
                page_name,
                block_uuid: uuid,
            })
        } else {
            Ok(DeepLinkTarget::Page {
                graph_id: None,
                page_name,
            })
        }
    }

    /// Parse a quilt://graph/{id}/page/{name} URL (host is "graph", segments = [graph_id, "page", page_name])
    fn parse_graph_url(
        segments: &[&str],
        fragment_uuid: Option<Uuid>,
    ) -> Result<DeepLinkTarget, String> {
        // Need at least: graph_id, "page", page_name = 3 segments
        if segments.len() < 3 {
            return Err("Invalid URL: missing graph id or page name".to_string());
        }

        // segments[0] == graph_id
        // segments[1] == "page"
        // segments[2] == page_name

        let graph_id = Self::decode_percent(segments[0])?;
        let page_name = Self::decode_percent(segments[2])?;

        if page_name.is_empty() {
            return Err("Invalid URL: missing page name".to_string());
        }

        if let Some(uuid) = fragment_uuid {
            Ok(DeepLinkTarget::Block {
                graph_id: Some(graph_id),
                page_name,
                block_uuid: uuid,
            })
        } else {
            Ok(DeepLinkTarget::Page {
                graph_id: Some(graph_id),
                page_name,
            })
        }
    }

    /// Decode percent-encoded characters in a string
    fn decode_percent(s: &str) -> Result<String, String> {
        // If no percent encoding, return as-is
        if !s.contains('%') {
            return Ok(s.to_string());
        }

        // Manual percent decoding
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if hex.len() == 2 {
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        result.push(byte as char);
                        continue;
                    }
                }
                // If decode fails, keep the original % and hex
                result.push('%');
                result.push_str(&hex);
            } else {
                result.push(c);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_url() {
        let result = DeepLinkParser::parse("quilt://page/Home").unwrap();
        match result {
            DeepLinkTarget::Page {
                graph_id,
                page_name,
            } => {
                assert_eq!(graph_id, None);
                assert_eq!(page_name, "Home");
            }
            _ => panic!("Expected Page variant"),
        }
    }

    #[test]
    fn test_parse_page_url_with_encoded_name() {
        let result = DeepLinkParser::parse("quilt://page/My%20Page").unwrap();
        match result {
            DeepLinkTarget::Page {
                graph_id,
                page_name,
            } => {
                assert_eq!(graph_id, None);
                assert_eq!(page_name, "My Page");
            }
            _ => panic!("Expected Page variant"),
        }
    }

    #[test]
    fn test_parse_block_url() {
        let result =
            DeepLinkParser::parse("quilt://page/Notes#block-550e8400-e29b-41d4-a716-446655440000")
                .unwrap();
        match result {
            DeepLinkTarget::Block {
                graph_id,
                page_name,
                block_uuid,
            } => {
                assert_eq!(graph_id, None);
                assert_eq!(page_name, "Notes");
                assert_eq!(
                    block_uuid.to_string(),
                    "550e8400-e29b-41d4-a716-446655440000"
                );
            }
            _ => panic!("Expected Block variant"),
        }
    }

    #[test]
    fn test_parse_graph_page_url() {
        let result = DeepLinkParser::parse("quilt://graph/my-graph/page/Home").unwrap();
        match result {
            DeepLinkTarget::Page {
                graph_id,
                page_name,
            } => {
                assert_eq!(graph_id, Some("my-graph".to_string()));
                assert_eq!(page_name, "Home");
            }
            _ => panic!("Expected Page variant"),
        }
    }

    #[test]
    fn test_parse_graph_block_url() {
        let result = DeepLinkParser::parse(
            "quilt://graph/my-graph/page/Notes#block-550e8400-e29b-41d4-a716-446655440000",
        )
        .unwrap();
        match result {
            DeepLinkTarget::Block {
                graph_id,
                page_name,
                block_uuid,
            } => {
                assert_eq!(graph_id, Some("my-graph".to_string()));
                assert_eq!(page_name, "Notes");
                assert_eq!(
                    block_uuid.to_string(),
                    "550e8400-e29b-41d4-a716-446655440000"
                );
            }
            _ => panic!("Expected Block variant"),
        }
    }

    #[test]
    fn test_invalid_scheme() {
        let result = DeepLinkParser::parse("https://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported URL scheme"));
    }

    #[test]
    fn test_missing_page_name() {
        let result = DeepLinkParser::parse("quilt://page/");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing page name"));
    }

    #[test]
    fn test_unrecognized_path() {
        let result = DeepLinkParser::parse("quilt://invalid/path/page/Test");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unrecognized URL path"));
    }

    #[test]
    fn test_encoded_special_chars() {
        // Test %23 (#) decoding
        let result = DeepLinkParser::parse("quilt://page/Hello%23World");
        match result {
            Ok(DeepLinkTarget::Page { page_name, .. }) => {
                assert_eq!(page_name, "Hello#World");
            }
            _ => panic!("Expected Page variant"),
        }
    }
}
