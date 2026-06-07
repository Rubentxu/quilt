//! Value objects module

mod align;
mod asset_type;
mod block_format;
mod block_type;
mod journal_day;
mod priority;
mod property_value;
mod task_marker;
mod uuid;

pub use align::Align;
pub use asset_type::AssetType;
pub use block_format::BlockFormat;
pub use block_type::BlockType;
pub use journal_day::JournalDay;
pub use priority::Priority;
pub use property_value::{PropertyValue, parse_properties};
pub use task_marker::TaskMarker;
pub use uuid::Uuid;

/// Trait for types that can be converted to a string representation
pub trait AsDisplayString {
    fn as_str(&self) -> String;
}
