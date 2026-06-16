//! Morning Briefing module
//!
//! Aggregates a daily snapshot of the user's knowledge graph including:
//! - Today's agenda (journal blocks from today)
//! - Decay alerts (stale blocks that need attention)
//! - Serendipity highlights (unexpected connections discovered)

pub mod engine;
pub mod types;

pub use engine::MorningBriefing;
pub use types::{AgendaItem, DecayAlert, MorningBriefingDto, SerendipityHighlight};
