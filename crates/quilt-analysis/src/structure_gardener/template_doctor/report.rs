//! Gardener integration types — extension points the
//! `StructureGardener` runs as part of its regular inspection cycle.
//!
//! `TemplateCare` runs the `TemplateDoctor` against a single
//! template. It is *not* an async task — the caller is expected to
//! have already loaded the template state. The `TemplateDoctor` is
//! pure, so the gardener composes it with its own I/O layer.

use super::TemplateDoctor;

/// Extension points the `StructureGardener` runs as part of its
/// regular inspection cycle.
#[derive(Debug, Clone)]
pub enum GardenerCare {
    /// Run the doctor against a specific template.
    TemplateCare(TemplateDoctor),
}

impl GardenerCare {
    /// Convenience constructor for the `TemplateCare` variant.
    pub fn template_care(doctor: TemplateDoctor) -> Self {
        GardenerCare::TemplateCare(doctor)
    }
}
