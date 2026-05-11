//! AssetType value object - type of embedded asset

use std::fmt;

/// AssetType represents the type of an embedded asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    /// Image (png, jpg, gif, svg, etc.)
    Image,
    /// PDF document
    Pdf,
    /// Audio file
    Audio,
    /// Video file
    Video,
    /// Other/unknown type
    Other,
}

impl AssetType {
    /// Get the MIME type prefix
    pub fn mime_prefix(&self) -> &'static str {
        match self {
            AssetType::Image => "image",
            AssetType::Pdf => "application",
            AssetType::Audio => "audio",
            AssetType::Video => "video",
            AssetType::Other => "application",
        }
    }

    /// Detect from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" => AssetType::Image,
            "pdf" => AssetType::Pdf,
            "mp3" | "wav" | "ogg" | "m4a" | "flac" => AssetType::Audio,
            "mp4" | "webm" | "mov" | "avi" | "mkv" => AssetType::Video,
            _ => AssetType::Other,
        }
    }

    /// Detect from MIME type
    pub fn from_mime(mime: &str) -> Option<Self> {
        if mime.starts_with("image/") {
            Some(AssetType::Image)
        } else if mime == "application/pdf" {
            Some(AssetType::Pdf)
        } else if mime.starts_with("audio/") {
            Some(AssetType::Audio)
        } else if mime.starts_with("video/") {
            Some(AssetType::Video)
        } else {
            None
        }
    }

    /// Check if this is an image type
    pub fn is_image(&self) -> bool {
        matches!(self, AssetType::Image)
    }

    /// Check if this is a document type
    pub fn is_document(&self) -> bool {
        matches!(self, AssetType::Pdf)
    }

    /// Check if this is a media type (audio or video)
    pub fn is_media(&self) -> bool {
        matches!(self, AssetType::Audio | AssetType::Video)
    }
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetType::Image => write!(f, "image"),
            AssetType::Pdf => write!(f, "pdf"),
            AssetType::Audio => write!(f, "audio"),
            AssetType::Video => write!(f, "video"),
            AssetType::Other => write!(f, "other"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_extension() {
        assert_eq!(AssetType::from_extension("png"), AssetType::Image);
        assert_eq!(AssetType::from_extension("jpg"), AssetType::Image);
        assert_eq!(AssetType::from_extension("pdf"), AssetType::Pdf);
        assert_eq!(AssetType::from_extension("mp3"), AssetType::Audio);
        assert_eq!(AssetType::from_extension("mp4"), AssetType::Video);
    }

    #[test]
    fn test_from_mime() {
        assert_eq!(AssetType::from_mime("image/png"), Some(AssetType::Image));
        assert_eq!(
            AssetType::from_mime("application/pdf"),
            Some(AssetType::Pdf)
        );
        assert_eq!(AssetType::from_mime("audio/mpeg"), Some(AssetType::Audio));
        assert_eq!(AssetType::from_mime("video/mp4"), Some(AssetType::Video));
    }

    #[test]
    fn test_type_checks() {
        assert!(AssetType::Image.is_image());
        assert!(!AssetType::Image.is_media());
        assert!(AssetType::Pdf.is_document());
        assert!(AssetType::Audio.is_media());
        assert!(AssetType::Video.is_media());
    }
}
