//! Template Doctor — diagnose and repair template contract drift (Q031 ROADMAP).
//!
//! A `TemplateDoctor` inspects a template's *current* state against its
//! declared `TemplateContract` and reports — and optionally repairs — every
//! way the template has drifted away from the contract.
//!
//! The doctor is **pure**: it takes a contract plus a snapshot of the
//! template's current state (properties + version) and produces reports.
//! It does not touch repositories. The `StructureGardener` (or any other
//! caller) is responsible for loading the template from storage and
//! persisting any repairs.
//!
//! ## Detected issues
//!
//! 1. **Missing required property** — the contract requires a property
//!    that the template does not currently carry.
//! 2. **Extra property** — the template carries a property that the
//!    contract does not declare.
//! 3. **Locked property changed** — a locked property's current value
//!    differs from the contract's canonical value.
//! 4. **Version mismatch** — the template's version is older than the
//!    contract's version (the contract has been bumped since the
//!    template was last applied).
//! 5. **Missing property default** — a required property has no
//!    default value, so the doctor cannot repair a missing entry
//!    without help from a human (reported as unfixable).
//!
//! ## Repairs
//!
//! - **Added** — a missing required property was added using its
//!   declared default.
//! - **Removed** — an extra property was removed.
//! - **Restored** — a locked property's value was restored to the
//!   contract's canonical value.
//! - **Upgraded** — the template's version was bumped to match the
//!   contract's version.
//!
//! Issues with no safe automatic fix (e.g. missing required property
//! without a default) appear in the `DiagnosisReport.issues` list and
//! in `RepairReport.unfixable` — they are never silently dropped.
//!
//! ## Module layout
//!
//! The doctor is split across sub-modules to keep each concern focused:
//!
//! - [`issue_types`] — data types: `TemplateState`, `Issue`, `IssueKind`,
//!   `DiagnosisReport`, `RepairAction`, `RepairReport`, `DefaultRegistry`.
//! - [`diagnosis`] — pure diagnosis logic (`diagnose`,
//!   `diagnose_with_canonical_locked_values`).
//! - [`repair`] — pure repair logic (`plan_repairs`, `repair`,
//!   `repair_with_canonical`).
//! - [`report`] — gardener integration types (`GardenerCare`).

pub mod diagnosis;
pub mod issue_types;
pub mod repair;
pub mod report;

pub use issue_types::{
    DefaultRegistry, DiagnosisReport, Issue, IssueKind, RepairAction, RepairReport, TemplateState,
};
pub use report::GardenerCare;

/// The Template Doctor. Pure inspector — no I/O, no side effects on
/// construction. Callers drive it with a contract + a `TemplateState`
/// and use the returned reports to update storage.
#[derive(Debug, Clone, Default)]
pub struct TemplateDoctor {
    /// Per-property defaults the doctor uses to repair
    /// `MissingRequiredProperty` issues. See `DefaultRegistry`.
    defaults: DefaultRegistry,
}

impl TemplateDoctor {
    /// Construct a doctor with no registered defaults. The doctor
    /// will report any `MissingRequiredProperty` as unfixable.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a doctor that uses the supplied registry of
    /// defaults to repair `MissingRequiredProperty` issues.
    pub fn with_defaults(defaults: DefaultRegistry) -> Self {
        Self { defaults }
    }

    /// Borrow the default registry.
    pub fn defaults(&self) -> &DefaultRegistry {
        &self.defaults
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::{PropertyKey, TemplateContract, Version};
    use quilt_domain::value_objects::Uuid;
    use std::collections::HashMap;

    fn uuid_from_u8(i: u8) -> Uuid {
        let mut b = [0u8; 16];
        b[0] = i;
        Uuid::from_bytes(b)
    }

    fn contract_with(
        template_id: Uuid,
        required: &[&str],
        locked: &[&str],
        version: Version,
    ) -> TemplateContract {
        let mut builder = TemplateContract::builder()
            .template_id(template_id)
            .version(version);
        for r in required {
            if locked.contains(r) {
                builder = builder.required_property(r).locked_layout(r);
            } else {
                builder = builder.required_property(r).inline_layout(r);
            }
        }
        builder.build().expect("contract build")
    }

    fn state_with(template_id: Uuid, pairs: &[(&str, &str)], version: Version) -> TemplateState {
        let mut map = HashMap::new();
        for (k, v) in pairs {
            map.insert((*k).to_string(), (*v).to_string());
        }
        TemplateState::new(template_id, map, version)
    }

    // ── 1. Missing required property ────────────────────────────

    #[test]
    fn missing_required_property_is_detected() {
        let tid = uuid_from_u8(1);
        let contract = contract_with(tid, &["title", "status"], &[], Version::new());
        // Template is missing 'status'.
        let state = state_with(tid, &[("title", "Hello")], Version::new());
        let doctor = TemplateDoctor::new();
        let report = doctor.diagnose(&contract, &state);

        assert!(!report.is_clean());
        let missing: Vec<&Issue> = report
            .issues
            .iter()
            .filter(|i| i.kind == IssueKind::MissingRequiredProperty)
            .collect();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].property.as_ref().unwrap().as_str(), "status");
        assert!(!missing[0].fixable);
    }

    #[test]
    fn missing_required_property_with_default_is_fixable_and_repaired() {
        let tid = uuid_from_u8(2);
        let contract = contract_with(tid, &["title", "status"], &[], Version::new());
        let state = state_with(tid, &[("title", "Hello")], Version::new());
        let doctor =
            TemplateDoctor::with_defaults(DefaultRegistry::new().with_default("status", "draft"));
        let report = doctor.diagnose(&contract, &state);

        let missing: Vec<&Issue> = report
            .issues
            .iter()
            .filter(|i| i.kind == IssueKind::MissingRequiredProperty)
            .collect();
        assert_eq!(missing.len(), 1);
        assert!(missing[0].fixable);

        let repair = doctor.repair(&contract, &state, &report);
        assert!(!repair.clean);
        assert_eq!(repair.actions.len(), 1);
        assert!(
            !repair
                .unfixable
                .iter()
                .any(|i| i.kind == IssueKind::MissingRequiredProperty)
        );
        match &repair.actions[0] {
            RepairAction::Added { key, value } => {
                assert_eq!(key.as_str(), "status");
                assert_eq!(value, "draft");
            }
            other => panic!("expected Added, got {other:?}"),
        }
        // Resulting state carries the new property.
        assert_eq!(
            repair
                .resulting_state
                .properties
                .get(&PropertyKey::new("status").unwrap()),
            Some(&"draft".to_string())
        );
    }

    // ── 2. Extra property ───────────────────────────────────────

    #[test]
    fn extra_property_is_detected_and_removed() {
        let tid = uuid_from_u8(3);
        let contract = contract_with(tid, &["title"], &[], Version::new());
        // Template carries an extra 'foo' property.
        let state = state_with(tid, &[("title", "Hello"), ("foo", "bar")], Version::new());
        let doctor = TemplateDoctor::new();
        let report = doctor.diagnose(&contract, &state);

        let extras: Vec<&Issue> = report
            .issues
            .iter()
            .filter(|i| i.kind == IssueKind::ExtraProperty)
            .collect();
        assert_eq!(extras.len(), 1);
        assert_eq!(extras[0].property.as_ref().unwrap().as_str(), "foo");
        assert!(extras[0].fixable);

        let repair = doctor.repair(&contract, &state, &report);
        match &repair.actions[0] {
            RepairAction::Removed { key } => assert_eq!(key.as_str(), "foo"),
            other => panic!("expected Removed, got {other:?}"),
        }
        assert!(
            !repair
                .resulting_state
                .properties
                .contains_key(&PropertyKey::new("foo").unwrap())
        );
    }

    // ── 3. Locked property changed ──────────────────────────────

    #[test]
    fn locked_property_changed_is_detected_and_restored() {
        let tid = uuid_from_u8(4);
        let contract = contract_with(tid, &["type"], &["type"], Version::new());
        // Template has 'type' = "user-modified" but contract says it should be "task".
        let state = state_with(tid, &[("type", "user-modified")], Version::new());
        let doctor = TemplateDoctor::new();
        let mut canonical = HashMap::new();
        canonical.insert(PropertyKey::new("type").unwrap(), "task".to_string());

        let report = doctor.diagnose_with_canonical_locked_values(&contract, &state, &canonical);
        let changed: Vec<&Issue> = report
            .issues
            .iter()
            .filter(|i| i.kind == IssueKind::LockedPropertyChanged)
            .collect();
        assert_eq!(changed.len(), 1);
        assert!(changed[0].fixable);

        let repair = doctor.repair_with_canonical(&contract, &state, &canonical);
        match &repair.actions[0] {
            RepairAction::Restored { key, value } => {
                assert_eq!(key.as_str(), "type");
                assert_eq!(value, "task");
            }
            other => panic!("expected Restored, got {other:?}"),
        }
        assert_eq!(
            repair
                .resulting_state
                .properties
                .get(&PropertyKey::new("type").unwrap()),
            Some(&"task".to_string())
        );
    }

    #[test]
    fn locked_property_at_contract_value_is_not_flagged() {
        let tid = uuid_from_u8(5);
        let contract = contract_with(tid, &["type"], &["type"], Version::new());
        let state = state_with(tid, &[("type", "task")], Version::new());
        let mut canonical = HashMap::new();
        canonical.insert(PropertyKey::new("type").unwrap(), "task".to_string());

        let doctor = TemplateDoctor::new();
        let report = doctor.diagnose_with_canonical_locked_values(&contract, &state, &canonical);
        assert!(
            report.is_clean(),
            "no drift expected, got {:?}",
            report.issues
        );
    }

    // ── 4. Version mismatch ──────────────────────────────────────

    #[test]
    fn version_mismatch_is_detected_and_upgraded() {
        let tid = uuid_from_u8(6);
        let contract = contract_with(tid, &["title"], &[], Version::from_u32(3));
        let state = state_with(tid, &[("title", "Hello")], Version::from_u32(1));
        let doctor = TemplateDoctor::new();

        let report = doctor.diagnose(&contract, &state);
        let vm: Vec<&Issue> = report
            .issues
            .iter()
            .filter(|i| i.kind == IssueKind::VersionMismatch)
            .collect();
        assert_eq!(vm.len(), 1);
        assert!(vm[0].fixable);
        assert!(!vm[0].is_property_scoped());

        let repair = doctor.repair(&contract, &state, &report);
        match &repair.actions[0] {
            RepairAction::Upgraded { new_version } => {
                assert_eq!(new_version.as_u32(), 3);
            }
            other => panic!("expected Upgraded, got {other:?}"),
        }
        assert_eq!(repair.resulting_state.version.as_u32(), 3);
    }

    #[test]
    fn matching_version_is_clean() {
        let tid = uuid_from_u8(7);
        let contract = contract_with(tid, &["title"], &[], Version::from_u32(2));
        let state = state_with(tid, &[("title", "Hello")], Version::from_u32(2));
        let doctor = TemplateDoctor::new();
        let report = doctor.diagnose(&contract, &state);
        assert!(report.is_clean());
    }

    // ── 5. Multiple issues ──────────────────────────────────────

    #[test]
    fn multiple_issues_all_detected_and_repaired() {
        let tid = uuid_from_u8(8);
        // Required: title, status, type. Locked: type.
        let contract = contract_with(
            tid,
            &["title", "status", "type"],
            &["type"],
            Version::from_u32(2),
        );
        // Template:
        //   - missing 'status'
        //   - extra 'unused'
        //   - 'type' changed from 'task' to 'user-changed'
        //   - version 1 vs contract 2
        let state = state_with(
            tid,
            &[
                ("title", "Hello"),
                ("type", "user-changed"),
                ("unused", "x"),
            ],
            Version::from_u32(1),
        );
        let doctor =
            TemplateDoctor::with_defaults(DefaultRegistry::new().with_default("status", "draft"));
        let mut canonical = HashMap::new();
        canonical.insert(PropertyKey::new("type").unwrap(), "task".to_string());

        let report = doctor.diagnose_with_canonical_locked_values(&contract, &state, &canonical);

        let kinds: Vec<IssueKind> = report.issues.iter().map(|i| i.kind).collect();
        assert!(kinds.contains(&IssueKind::MissingRequiredProperty));
        assert!(kinds.contains(&IssueKind::ExtraProperty));
        assert!(kinds.contains(&IssueKind::LockedPropertyChanged));
        assert!(kinds.contains(&IssueKind::VersionMismatch));
        assert!(report.fixable_count >= 4);
        assert_eq!(report.unfixable_count, 0);

        let repair = doctor.repair_with_canonical(&contract, &state, &canonical);
        assert!(repair.unfixable.is_empty());
        // 4 actions: added status, removed unused, restored type, upgraded version.
        assert_eq!(repair.actions.len(), 4);

        // Resulting state is fully repaired.
        let result = &repair.resulting_state;
        assert_eq!(
            result.properties.get(&PropertyKey::new("title").unwrap()),
            Some(&"Hello".to_string())
        );
        assert_eq!(
            result.properties.get(&PropertyKey::new("status").unwrap()),
            Some(&"draft".to_string())
        );
        assert_eq!(
            result.properties.get(&PropertyKey::new("type").unwrap()),
            Some(&"task".to_string())
        );
        assert!(
            !result
                .properties
                .contains_key(&PropertyKey::new("unused").unwrap())
        );
        assert_eq!(result.version.as_u32(), 2);
    }

    // ── 6. No issues (clean) ─────────────────────────────────────

    #[test]
    fn clean_template_produces_clean_report() {
        let tid = uuid_from_u8(9);
        let contract = contract_with(tid, &["title", "status"], &[], Version::new());
        let state = state_with(
            tid,
            &[("title", "Hello"), ("status", "draft")],
            Version::new(),
        );
        let doctor = TemplateDoctor::new();
        let report = doctor.diagnose(&contract, &state);
        assert!(report.is_clean());
        assert_eq!(report.fixable_count, 0);
        assert_eq!(report.unfixable_count, 0);

        let repair = doctor.repair(&contract, &state, &report);
        assert!(repair.clean);
        assert_eq!(repair.actions.len(), 0);
        assert!(repair.unfixable.is_empty());
    }

    // ── 7. Unfixable issue ───────────────────────────────────────

    #[test]
    fn unfixable_missing_required_property_is_reported_as_unfixable() {
        let tid = uuid_from_u8(10);
        let contract = contract_with(tid, &["title", "status"], &[], Version::new());
        let state = state_with(tid, &[("title", "Hello")], Version::new());
        // Doctor with NO registered defaults.
        let doctor = TemplateDoctor::new();
        let report = doctor.diagnose(&contract, &state);

        // Both the missing-required and the missing-default issues
        // are reported; the missing-required one is unfixable
        // because there's no default to fill it with.
        let kinds: Vec<IssueKind> = report.issues.iter().map(|i| i.kind).collect();
        assert!(kinds.contains(&IssueKind::MissingRequiredProperty));
        assert!(kinds.contains(&IssueKind::MissingPropertyDefault));
        assert!(report.unfixable_count >= 2);

        let repair = doctor.repair(&contract, &state, &report);
        assert!(!repair.clean);
        assert_eq!(repair.actions.len(), 0);
        assert!(
            repair
                .unfixable
                .iter()
                .any(|i| i.kind == IssueKind::MissingRequiredProperty)
        );
        assert!(
            repair
                .unfixable
                .iter()
                .any(|i| i.kind == IssueKind::MissingPropertyDefault)
        );
        // The input state is unchanged because no action could be taken.
        assert_eq!(repair.resulting_state, state);
    }

    // ── Aux / contract mismatch guards ──────────────────────────

    #[test]
    #[should_panic(expected = "contract.template_id")]
    fn diagnose_panics_on_template_id_mismatch() {
        let contract = contract_with(uuid_from_u8(1), &["title"], &[], Version::new());
        let state = state_with(uuid_from_u8(2), &[("title", "x")], Version::new());
        let doctor = TemplateDoctor::new();
        let _ = doctor.diagnose(&contract, &state);
    }

    #[test]
    fn default_registry_normalizes_keys() {
        let r = DefaultRegistry::new()
            .with_default("Status", "draft")
            .with_default("  type  ", "task");
        assert_eq!(r.get(&PropertyKey::new("status").unwrap()), Some("draft"));
        assert_eq!(r.get(&PropertyKey::new("type").unwrap()), Some("task"));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn template_state_new_normalizes_keys() {
        let tid = uuid_from_u8(11);
        let mut props = HashMap::new();
        props.insert("  Status  ".to_string(), "draft".to_string());
        let state = TemplateState::new(tid, props, Version::new());
        assert!(
            state
                .properties()
                .contains_key(&PropertyKey::new("status").unwrap())
        );
    }

    #[test]
    fn partition_separates_fixable_from_unfixable() {
        let tid = uuid_from_u8(12);
        let contract = contract_with(tid, &["title", "status"], &[], Version::new());
        let state = state_with(tid, &[("title", "x"), ("extra", "y")], Version::new());
        let report = TemplateDoctor::new().diagnose(&contract, &state);
        let (fixable, unfixable) = report.partition();
        assert!(fixable.iter().any(|i| i.kind == IssueKind::ExtraProperty));
        assert!(
            unfixable
                .iter()
                .any(|i| i.kind == IssueKind::MissingRequiredProperty)
        );
        assert!(
            unfixable
                .iter()
                .any(|i| i.kind == IssueKind::MissingPropertyDefault)
        );
    }

    #[test]
    fn repair_action_property_accessor() {
        let key = PropertyKey::new("k").unwrap();
        assert_eq!(
            RepairAction::Added {
                key: key.clone(),
                value: "v".to_string()
            }
            .property(),
            Some(&key)
        );
        assert_eq!(
            RepairAction::Removed { key: key.clone() }.property(),
            Some(&key)
        );
        assert_eq!(
            RepairAction::Restored {
                key: key.clone(),
                value: "v".to_string()
            }
            .property(),
            Some(&key)
        );
        assert!(
            RepairAction::Upgraded {
                new_version: Version::new()
            }
            .property()
            .is_none()
        );
    }

    #[test]
    fn gardener_care_template_care_constructor() {
        let doctor = TemplateDoctor::new();
        let care = GardenerCare::template_care(doctor.clone());
        match care {
            GardenerCare::TemplateCare(d) => assert_eq!(d.defaults().len(), 0),
        }
    }
}
