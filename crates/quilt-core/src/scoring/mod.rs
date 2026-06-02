//! Connection scoring algorithms.
//!
//! Pure functions for computing structural similarity (Jaccard),
//! temporal decay (halflife), and composite scoring.
//!
//! These are extracted from the duplicated implementations in:
//! - `quilt-analysis/src/connection_engine/engine.rs`
//! - `quilt-cognitive/src/serendipity/engine.rs`

pub mod connection;
