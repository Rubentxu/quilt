//! Heuristic NL-to-DSL transformer (Intent Search V3a)
//!
//! Rule-based pipeline: tokenize → match rules → generate QueryAst.
//! NO LLM — pure pattern matching with confidence scoring.
//!
//! # Architecture
//!
//! Each rule implements [`IntentRule`] with `matches(&str) -> Option<(QueryAst, f32)>`.
//! The [`HeuristicEngine`] runs all rules and picks the highest-confidence match.
//!
//! # Graph-Aware Rules (F3)
//!
//! Graph-aware rules require access to a petgraph `DiGraph`. Pass `Option<&DiGraph>`
//! to [`HeuristicEngine::parse_with_graph`] to enable these rules. Text-only queries
//! work without a graph (rules return None).

mod shared;
mod types;
mod rules;
mod engine;

// Re-export public API
pub use types::{GraphResult, GraphResultType, IntentResult, IntentRule};
pub use rules::task_status::TaskStatusRule;
pub use rules::temporal::TemporalRule;
pub use rules::group_by::GroupByRule;
pub use rules::project_filter::ProjectFilterRule;
pub use rules::review_status::ReviewStatusRule;
pub use rules::created_by::CreatedByRule;
pub use rules::combined::CombinedRule;
pub use rules::related_to::RelatedToRule;
pub use rules::connected_to::ConnectedToRule;
pub use rules::most_central::MostCentralRule;
pub use rules::path_between::PathBetweenRule;
pub use engine::HeuristicEngine;
