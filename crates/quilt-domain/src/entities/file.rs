//! File entity - represents a file in the file system

use crate::value_objects::Uuid;

/// File represents a file in the graph's file system.
///
/// Files can be:
/// - Markdown/Org pages stored on disk
/// - Assets (images, PDFs, etc.)
/// - Configuration files
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// Unique identifier
    id: Uuid,
    /// Relative path from the graph root
    path: String,
    /// File content (for text files)
    content: Option<String>,
    /// SHA256 hash of the file content
    hash: Vec<u8>,
    /// File size in bytes
    size_bytes: i64,
    /// MIME type (if known)
    mime_type: Option<String>,
    /// Creation timestamp
    created_at: chrono::DateTime<chrono::Utc>,
    /// Last modification timestamp
    last_modified_at: chrono::DateTime<chrono::Utc>,
}

impl File {
    /// Create a new file record
    pub fn new(
        path: impl Into<String>,
        content: Option<String>,
        mime_type: Option<String>,
    ) -> Self {
        let path = path.into();
        let content_clone = content.clone();
        let (hash, size_bytes) = Self::compute_hash_and_size(&content_clone);

        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            path,
            content,
            hash,
            size_bytes,
            mime_type,
            created_at: now,
            last_modified_at: now,
        }
    }

    /// Create a file from raw bytes
    pub fn from_bytes(path: impl Into<String>, data: Vec<u8>, mime_type: Option<String>) -> Self {
        let path = path.into();
        let hash = Self::sha256(&data);
        let size_bytes = data.len() as i64;
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            path,
            content: None,
            hash,
            size_bytes,
            mime_type,
            created_at: now,
            last_modified_at: now,
        }
    }

    /// Compute SHA256 hash and size from content
    fn compute_hash_and_size(content: &Option<String>) -> (Vec<u8>, i64) {
        match content {
            Some(text) => {
                let bytes = text.as_bytes();
                (Self::sha256(bytes), bytes.len() as i64)
            }
            None => (Vec::new(), 0),
        }
    }

    /// Compute SHA256 hash
    fn sha256(data: &[u8]) -> Vec<u8> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        hash.to_be_bytes().to_vec()
    }

    /// Get the file ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the file path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the file content
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Get the file size
    pub fn size(&self) -> i64 {
        self.size_bytes
    }

    /// Get the MIME type
    pub fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }

    /// Check if this is a text file
    pub fn is_text(&self) -> bool {
        self.mime_type
            .as_ref()
            .is_some_and(|m| m.starts_with("text/"))
            || self.path.ends_with(".md")
            || self.path.ends_with(".org")
            || self.path.ends_with(".txt")
    }

    /// Check if this is an image
    pub fn is_image(&self) -> bool {
        self.mime_type
            .as_ref()
            .is_some_and(|m| m.starts_with("image/"))
            || self.path.ends_with(".png")
            || self.path.ends_with(".jpg")
            || self.path.ends_with(".jpeg")
            || self.path.ends_with(".gif")
            || self.path.ends_with(".svg")
    }

    /// Check if this is a PDF
    pub fn is_pdf(&self) -> bool {
        self.mime_type
            .as_ref()
            .is_some_and(|m| m == "application/pdf")
            || self.path.ends_with(".pdf")
    }

    /// Update the content and recompute hash
    pub fn update_content(&mut self, content: String) {
        self.content = Some(content.clone());
        self.hash = Self::sha256(content.as_bytes());
        self.size_bytes = content.len() as i64;
        self.last_modified_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_creation() {
        let file = File::new(
            "pages/test.md",
            Some("# Hello".to_string()),
            Some("text/markdown".to_string()),
        );

        assert_eq!(file.path(), "pages/test.md");
        assert!(file.is_text());
        assert!(!file.is_image());
    }

    #[test]
    fn test_image_detection() {
        let file = File::from_bytes(
            "assets/logo.png",
            vec![0, 1, 2],
            Some("image/png".to_string()),
        );

        assert!(file.is_image());
        assert!(!file.is_text());
    }
}
