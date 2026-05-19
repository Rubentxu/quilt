//! HTTP Route Handlers
//!
//! This module contains handlers for all REST endpoints.

pub mod blocks;
pub mod cognitive;
pub mod events;
pub mod graph;
pub mod pages;
pub mod search;

pub use blocks::routes as blocks_routes;
pub use cognitive::routes as cognitive_routes;
pub use events::routes as events_routes;
pub use graph::routes as graph_routes;
pub use pages::routes as pages_routes;
pub use search::routes as search_routes;
