//! V1 projection contracts — 6 pure-function implementations.
//!
//! Mirrors `quilt-application::services::projection::contracts` (slice #4)
//! but uses pure functions (no `Arc<dyn Fn>` closures, which cannot
//! cross the WASM boundary). The contracts are the 6 V1 projection
//! shapes: `task`, `heading`, `media`, `date`, `link`, `default`.
//!
//! # V1 contract priorities
//!
//! | Priority     | Contract   |
//! |--------------|------------|
//! | 100          | `task`     |
//! | 150          | `heading`  |
//! | 200          | `media`    |
//! | 250          | `date`     |
//! | 300          | `link`     |
//! | `u32::MAX`   | `default`  |
//!
//! # Deviations from server semantics (documented)
//!
//! - **Heading** uses `IsOneOf(heading-level, [1, 2, 3])` (tightened
//!   from server's `IsSet`); HTTP path is the fallback for unusual
//!   levels (which the V1 canonicalizer never emits).
//! - **Link** uses `IsSet("link")` (URL presence) instead of
//!   `IsSet("link-kind")`; the BlockRow needs the URL to render the
//!   affordance.
//!
//! See `openspec/changes/adr-0028-wasm-property-value-projection/specs/wasm-projection-contracts/spec.md`
//! for the full rationale.

pub mod date;
pub mod default;
pub mod heading;
pub mod link;
pub mod media;
pub mod task;

pub use date::DateContract;
pub use default::DefaultContract;
pub use heading::HeadingContract;
pub use link::LinkContract;
pub use media::MediaContract;
pub use task::TaskContract;

/// Helper: returns true if `properties[key]` is a string in `allowed`.
///
/// Used by the V1 task contract to check `status:: ∈ {todo, in-progress,
/// done, cancelled, waiting}`. The string-equality comparison is
/// byte-equal to the server's `PropertyPredicate::IsOneOf` evaluation
/// for `PropertyValue::String` values.
pub(crate) fn match_status_one_of(
    properties: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    allowed: &[&str],
) -> bool {
    properties
        .get(key)
        .and_then(|v| v.as_str())
        .map_or(false, |s| allowed.contains(&s))
}
