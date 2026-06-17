//! Domain entities

mod annotation;
mod asset;
mod block;
mod file;
pub mod global_app_state;
mod graph_space;
mod journal;
mod page;
mod tag;
mod template_contract;
mod user_settings;

pub use annotation::{Annotation, AnnotationCreate, AnnotationScope, AnnotationStatus, AuthorType};
pub use asset::Asset;
pub use block::{Block, BlockCreate, BlockUpdate};
pub use file::File;
pub use global_app_state::{GlobalAppState, RECENTS_CAP};
pub use graph_space::GraphSpace;
pub use journal::Journal;
pub use page::{Page, PageCreate};
pub use tag::Tag;
pub use template_contract::{
    ContractError, PropertyKey, TemplateContract, TemplateContractBuilder, TemplateLayout, Version,
};
pub use user_settings::UserSettings;
