//! Structure Gardener
//!
//! Tracks belief evolution in journal pages over time, detects contradictions,
//! and suggests areas for deeper exploration.
//!
//! Sub-modules:
//! - `engine` — original belief/contradiction engine.
//! - `template_doctor` — Q031 Template Doctor: diagnoses and repairs
//!   template-contract drift. Composes with the gardener via
//!   `GardenerCare::TemplateCare`.

pub mod engine;
pub mod template_doctor;
pub mod types;

pub use engine::{StructureGardener, StructureGardenerError};
pub use template_doctor::{
    DefaultRegistry, DiagnosisReport, GardenerCare, Issue, IssueKind, RepairAction, RepairReport,
    TemplateDoctor, TemplateState,
};
pub use types::*;
