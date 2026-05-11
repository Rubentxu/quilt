//! Asset entity - represents an embedded asset in a block

use crate::value_objects::Uuid;
use crate::value_objects::{Align, AssetType};

/// Asset represents an embedded asset (image, PDF, video, etc.) in a block.
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    /// The block this asset is embedded in
    block_id: Uuid,
    /// The file ID of the asset
    file_id: Uuid,
    /// Type of asset
    asset_type: AssetType,
    /// Display width (for images)
    width: Option<i32>,
    /// Display height (for images)
    height: Option<i32>,
    /// Display alignment
    align: Align,
    /// External URL (for linked assets, not uploaded)
    external_url: Option<String>,
}

impl Asset {
    /// Create a new asset embedded in a block
    pub fn new(block_id: Uuid, file_id: Uuid, asset_type: AssetType) -> Self {
        Self {
            block_id,
            file_id,
            asset_type,
            width: None,
            height: None,
            align: Align::Center,
            external_url: None,
        }
    }

    /// Create an external URL asset (not uploaded)
    pub fn external(block_id: Uuid, url: impl Into<String>) -> Self {
        Self {
            block_id,
            file_id: Uuid::new_v4(),
            asset_type: AssetType::Image,
            width: None,
            height: None,
            align: Align::Center,
            external_url: Some(url.into()),
        }
    }

    /// Get the block ID
    pub fn block_id(&self) -> Uuid {
        self.block_id
    }

    /// Get the file ID
    pub fn file_id(&self) -> Uuid {
        self.file_id
    }

    /// Get the asset type
    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    /// Check if this is an external URL
    pub fn is_external(&self) -> bool {
        self.external_url.is_some()
    }

    /// Set dimensions (for images)
    pub fn with_dimensions(mut self, width: i32, height: i32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set alignment
    pub fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Get the aspect ratio (width/height) if available
    pub fn aspect_ratio(&self) -> Option<f64> {
        match (self.width, self.height) {
            (Some(w), Some(h)) if h > 0 => Some(w as f64 / h as f64),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_creation() {
        let block_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();

        let asset = Asset::new(block_id, file_id, AssetType::Image)
            .with_dimensions(800, 600)
            .with_align(Align::Left);

        assert_eq!(asset.block_id(), block_id);
        assert_eq!(asset.file_id(), file_id);
        assert!(asset.aspect_ratio().is_some());
        assert_eq!(asset.aspect_ratio(), Some(800.0 / 600.0));
    }

    #[test]
    fn test_external_asset() {
        let block_id = Uuid::new_v4();
        let asset = Asset::external(block_id, "https://example.com/image.png");

        assert!(asset.is_external());
    }
}
