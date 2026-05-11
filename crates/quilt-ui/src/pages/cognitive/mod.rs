//! Cognitive pages module
//!
//! Provides pages for cognitive analysis features:
//! - CognitiveDashboard: Overview of cognitive state
//! - SerendipityFeed: Unexpected connections discovery
//! - ArgumentMapView: Argument structure visualization
//! - MentalModelGarden: Belief evolution tracking

pub mod argument_map;
pub mod dashboard;
pub mod mental_model;
pub mod serendipity;

pub use argument_map::ArgumentMapView;
pub use dashboard::CognitiveDashboard;
pub use mental_model::MentalModelGarden;
pub use serendipity::SerendipityFeed;
