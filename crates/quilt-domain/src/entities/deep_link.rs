//! DeepLink entity - represents a deep link between documents
//!
//! Deep links allow blocks and pages to reference other blocks, pages,
//! or external URLs with optional link text and metadata.

use crate::errors::DomainError;
use crate::value_objects::Uuid;

/// Link type enumeration - the kind of target a deep link points to
#[derive(Debug, Clone, PartialEq)]
pub enum LinkType {
    /// Internal link to a block
    InternalBlock,
    /// Internal link to a page
    InternalPage,
    /// External URL link
    ExternalUrl,
}

impl LinkType {
    /// Convert to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::InternalBlock => "block",
            LinkType::InternalPage => "page",
            LinkType::ExternalUrl => "url",
        }
    }

    /// Parse from string
    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "block" => Some(LinkType::InternalBlock),
            "page" => Some(LinkType::InternalPage),
            "url" => Some(LinkType::ExternalUrl),
            _ => None,
        }
    }
}

/// DeepLink represents a link from a source block/page to a target.
///
/// # Fields
/// - `id`: Unique identifier for this deep link
/// - `source_id`: The block or page that contains this link
/// - `source_type`: Whether the source is a block or page
/// - `target_id`: The target block/page ID (None for external URLs)
/// - `link_type`: The type of target (internal block, internal page, external URL)
/// - `external_url`: The target URL (for external links)
/// - `link_text`: The display text for the link (from the content)
/// - `context`: Optional surrounding text for link context
/// - `created_at`: When the link was created
#[derive(Debug, Clone, PartialEq)]
pub struct DeepLink {
    /// Unique identifier
    pub id: Uuid,
    /// Source block or page ID
    pub source_id: Uuid,
    /// Whether the source is a block or page
    pub source_type: LinkSourceType,
    /// Target block/page ID (None for external URLs)
    pub target_id: Option<Uuid>,
    /// Target page name (for internal page links)
    pub target_page_name: Option<String>,
    /// The type of link target
    pub link_type: LinkType,
    /// External URL (for external links)
    pub external_url: Option<String>,
    /// Display text for the link
    pub link_text: Option<String>,
    /// Surrounding context for the link
    pub context: Option<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// What type of entity is the source
#[derive(Debug, Clone, PartialEq)]
pub enum LinkSourceType {
    /// Source is a block
    Block,
    /// Source is a page
    Page,
}

impl LinkSourceType {
    /// Convert to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkSourceType::Block => "block",
            LinkSourceType::Page => "page",
        }
    }

    /// Parse from string
    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "block" => Some(LinkSourceType::Block),
            "page" => Some(LinkSourceType::Page),
            _ => None,
        }
    }
}

/// Data required to create a new deep link
#[derive(Debug, Clone)]
pub struct DeepLinkCreate {
    pub source_id: Uuid,
    pub source_type: LinkSourceType,
    pub target_id: Option<Uuid>,
    pub target_page_name: Option<String>,
    pub link_type: LinkType,
    pub external_url: Option<String>,
    pub link_text: Option<String>,
    pub context: Option<String>,
}

impl DeepLink {
    /// Create a new deep link
    pub fn new(create: DeepLinkCreate) -> Result<Self, DomainError> {
        // Validate external URL is provided for ExternalUrl links
        if create.link_type == LinkType::ExternalUrl && create.external_url.is_none() {
            return Err(DomainError::InvalidData(
                "External URL is required for external links".to_string(),
            ));
        }

        // Validate target for internal links
        if create.link_type == LinkType::InternalBlock && create.target_id.is_none() {
            return Err(DomainError::InvalidData(
                "Target ID is required for internal block links".to_string(),
            ));
        }

        // For InternalPage links, target_page_name is required (not target_id)
        if create.link_type == LinkType::InternalPage && create.target_page_name.is_none() {
            return Err(DomainError::InvalidData(
                "Target page name is required for internal page links".to_string(),
            ));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            source_id: create.source_id,
            source_type: create.source_type,
            target_id: create.target_id,
            target_page_name: create.target_page_name,
            link_type: create.link_type,
            external_url: create.external_url,
            link_text: create.link_text,
            context: create.context,
            created_at: chrono::Utc::now(),
        })
    }

    /// Get the target identifier as a string (for display)
    pub fn target_identifier(&self) -> String {
        if let Some(url) = &self.external_url {
            url.clone()
        } else if let Some(page_name) = &self.target_page_name {
            format!("page:{}", page_name)
        } else if let Some(id) = &self.target_id {
            format!("block:{}", id)
        } else {
            "unknown".to_string()
        }
    }

    /// Check if this is an internal link
    pub fn is_internal(&self) -> bool {
        self.link_type != LinkType::ExternalUrl
    }

    /// Check if this is an external link
    pub fn is_external(&self) -> bool {
        self.link_type == LinkType::ExternalUrl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    #[test]
    fn test_deep_link_creation_internal_block() {
        let source_id = create_test_uuid();
        let target_id = Uuid::new_v4();

        let create = DeepLinkCreate {
            source_id,
            source_type: LinkSourceType::Block,
            target_id: Some(target_id),
            target_page_name: None,
            link_type: LinkType::InternalBlock,
            external_url: None,
            link_text: Some("Related concept".to_string()),
            context: Some("See also this topic".to_string()),
        };

        let link = DeepLink::new(create).unwrap();
        assert_eq!(link.source_id, source_id);
        assert_eq!(link.target_id, Some(target_id));
        assert_eq!(link.link_type, LinkType::InternalBlock);
        assert_eq!(link.link_text, Some("Related concept".to_string()));
        assert!(link.is_internal());
        assert!(!link.is_external());
    }

    #[test]
    fn test_deep_link_creation_external_url() {
        let source_id = create_test_uuid();

        let create = DeepLinkCreate {
            source_id,
            source_type: LinkSourceType::Block,
            target_id: None,
            target_page_name: None,
            link_type: LinkType::ExternalUrl,
            external_url: Some("https://example.com".to_string()),
            link_text: Some("External resource".to_string()),
            context: None,
        };

        let link = DeepLink::new(create).unwrap();
        assert_eq!(link.link_type, LinkType::ExternalUrl);
        assert_eq!(link.external_url, Some("https://example.com".to_string()));
        assert!(link.is_external());
        assert!(!link.is_internal());
    }

    #[test]
    fn test_deep_link_validation_missing_url() {
        let create = DeepLinkCreate {
            source_id: create_test_uuid(),
            source_type: LinkSourceType::Block,
            target_id: None,
            target_page_name: None,
            link_type: LinkType::ExternalUrl,
            external_url: None, // Missing URL for external link
            link_text: None,
            context: None,
        };

        let result = DeepLink::new(create);
        assert!(result.is_err());
    }

    #[test]
    fn test_deep_link_validation_missing_target() {
        let create = DeepLinkCreate {
            source_id: create_test_uuid(),
            source_type: LinkSourceType::Block,
            target_id: None, // Missing target for internal link
            target_page_name: None,
            link_type: LinkType::InternalBlock,
            external_url: None,
            link_text: None,
            context: None,
        };

        let result = DeepLink::new(create);
        assert!(result.is_err());
    }

    #[test]
    fn test_target_identifier_block() {
        let target_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let create = DeepLinkCreate {
            source_id: Uuid::new_v4(),
            source_type: LinkSourceType::Block,
            target_id: Some(target_id),
            target_page_name: None,
            link_type: LinkType::InternalBlock,
            external_url: None,
            link_text: None,
            context: None,
        };

        let link = DeepLink::new(create).unwrap();
        assert_eq!(link.target_identifier(), format!("block:{}", target_id));
    }

    #[test]
    fn test_target_identifier_page() {
        let create = DeepLinkCreate {
            source_id: Uuid::new_v4(),
            source_type: LinkSourceType::Page,
            target_id: None,
            target_page_name: Some("My Page".to_string()),
            link_type: LinkType::InternalPage,
            external_url: None,
            link_text: None,
            context: None,
        };

        let link = DeepLink::new(create).unwrap();
        assert_eq!(link.target_identifier(), "page:My Page");
    }

    #[test]
    fn test_target_identifier_external() {
        let create = DeepLinkCreate {
            source_id: Uuid::new_v4(),
            source_type: LinkSourceType::Block,
            target_id: None,
            target_page_name: None,
            link_type: LinkType::ExternalUrl,
            external_url: Some("https://rust-lang.org".to_string()),
            link_text: None,
            context: None,
        };

        let link = DeepLink::new(create).unwrap();
        assert_eq!(link.target_identifier(), "https://rust-lang.org");
    }

    #[test]
    fn test_link_type_from_str() {
        assert_eq!(LinkType::try_from_str("block"), Some(LinkType::InternalBlock));
        assert_eq!(LinkType::try_from_str("page"), Some(LinkType::InternalPage));
        assert_eq!(LinkType::try_from_str("url"), Some(LinkType::ExternalUrl));
        assert_eq!(LinkType::try_from_str("unknown"), None);
    }

    #[test]
    fn test_link_source_type_from_str() {
        assert_eq!(LinkSourceType::try_from_str("block"), Some(LinkSourceType::Block));
        assert_eq!(LinkSourceType::try_from_str("page"), Some(LinkSourceType::Page));
        assert_eq!(LinkSourceType::try_from_str("unknown"), None);
    }
}
