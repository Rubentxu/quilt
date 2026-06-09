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

use quilt_domain::entities::{PropertyKey, TemplateContract, Version};
use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Snapshot ────────────────────────────────────────────────────────

/// A read-only snapshot of a template's *current* state, as observed
/// by the doctor. Callers populate this from a `Page` (for `properties`)
/// and from the page's stored version metadata (for `version`).
///
/// The doctor treats the snapshot as the source of truth for "what
/// the template currently looks like". A repair produces a *new*
/// snapshot with the fixes applied; the caller is responsible for
/// persisting that back to storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateState {
    /// The template this snapshot is for. Must match the
    /// `TemplateContract::template_id`.
    pub template_id: Uuid,
    /// The template's current property set, keyed by normalized
    /// property name.
    pub properties: HashMap<PropertyKey, String>,
    /// The template's current contract version. The doctor compares
    /// this against `TemplateContract::version` to detect drift.
    pub version: Version,
}

impl TemplateState {
    /// Construct a snapshot from a property map (string keys) and a
    /// version. Keys are normalized via `PropertyKey::new`; invalid
    /// keys are silently dropped (consistent with the contract
    /// builder's behavior).
    pub fn new(
        template_id: Uuid,
        properties: HashMap<String, String>,
        version: Version,
    ) -> Self {
        let normalized = properties
            .into_iter()
            .filter_map(|(k, v)| PropertyKey::new(&k).ok().map(|key| (key, v)))
            .collect();
        Self {
            template_id,
            properties: normalized,
            version,
        }
    }

    /// Borrow the property map.
    pub fn properties(&self) -> &HashMap<PropertyKey, String> {
        &self.properties
    }

    /// Apply a list of repair actions to a clone of this snapshot,
    /// returning the repaired snapshot. This is the same logic the
    /// doctor uses internally; exposed for callers that want to
    /// preview a repair without invoking `repair()`.
    pub fn apply_actions(&self, actions: &[RepairAction]) -> TemplateState {
        let mut next = self.clone();
        for action in actions {
            match action {
                RepairAction::Added { key, value } => {
                    next.properties.insert(key.clone(), value.clone());
                }
                RepairAction::Removed { key } => {
                    next.properties.remove(key);
                }
                RepairAction::Restored { key, value } => {
                    next.properties.insert(key.clone(), value.clone());
                }
                RepairAction::Upgraded { new_version } => {
                    next.version = *new_version;
                }
            }
        }
        next
    }
}

// ── Issue model ─────────────────────────────────────────────────────

/// The kind of issue the doctor detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    /// A property declared as required in the contract is missing
    /// from the template.
    MissingRequiredProperty,
    /// The template has a property the contract does not declare.
    ExtraProperty,
    /// A locked property's current value differs from the
    /// contract's canonical value.
    LockedPropertyChanged,
    /// The template's version is older than the contract's version.
    VersionMismatch,
    /// A required property has no default value declared, so the
    /// doctor cannot auto-repair a missing entry. Always reported
    /// alongside `MissingRequiredProperty` and always unfixable.
    MissingPropertyDefault,
}

/// A single issue detected by the doctor.
///
/// `fixable == false` means the doctor knows how to detect the
/// problem but cannot safely produce a `RepairAction` for it. The
/// doctor still surfaces these in `DiagnosisReport` so the caller
/// can hand them off to a human or a higher-level policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    /// What kind of issue this is.
    pub kind: IssueKind,
    /// The property this issue is about, when applicable. `None` for
    /// `VersionMismatch`, which is template-wide.
    pub property: Option<PropertyKey>,
    /// Human-readable details (e.g. expected vs actual values).
    pub detail: String,
    /// Whether the doctor can produce a safe automatic fix.
    pub fixable: bool,
}

impl Issue {
    /// Quick predicate: does this issue have an associated property?
    pub fn is_property_scoped(&self) -> bool {
        self.property.is_some()
    }
}

/// The full diagnosis of a template: every issue detected, regardless
/// of whether it is fixable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosisReport {
    /// The template this report is for.
    pub template_id: Uuid,
    /// All detected issues, in detection order. May be empty (clean
    /// template).
    pub issues: Vec<Issue>,
    /// Convenience count of fixable issues.
    pub fixable_count: usize,
    /// Convenience count of unfixable issues.
    pub unfixable_count: usize,
}

impl DiagnosisReport {
    /// True when the doctor found no issues.
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// Borrow the issues, partitioned by `Issue::fixable`.
    pub fn partition(&self) -> (Vec<&Issue>, Vec<&Issue>) {
        let mut fixable = Vec::new();
        let mut unfixable = Vec::new();
        for issue in &self.issues {
            if issue.fixable {
                fixable.push(issue);
            } else {
                unfixable.push(issue);
            }
        }
        (fixable, unfixable)
    }
}

// ── Repair model ────────────────────────────────────────────────────

/// A single repair action the doctor has taken (or proposes to take).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RepairAction {
    /// A missing required property was added with the supplied value.
    Added {
        /// The property that was added.
        key: PropertyKey,
        /// The default value the doctor used.
        value: String,
    },
    /// An extra property was removed.
    Removed {
        /// The property that was removed.
        key: PropertyKey,
    },
    /// A locked property was restored to its contract value.
    Restored {
        /// The property that was restored.
        key: PropertyKey,
        /// The contract value the property was restored to.
        value: String,
    },
    /// The template's version was bumped to match the contract.
    Upgraded {
        /// The new version.
        new_version: Version,
    },
}

impl RepairAction {
    /// The property this action is about, if any.
    pub fn property(&self) -> Option<&PropertyKey> {
        match self {
            RepairAction::Added { key, .. }
            | RepairAction::Removed { key }
            | RepairAction::Restored { key, .. } => Some(key),
            RepairAction::Upgraded { .. } => None,
        }
    }
}

/// The result of running `repair()`: the actions taken, the
/// resulting snapshot, and any issues the doctor refused to auto-fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairReport {
    /// The template this report is for.
    pub template_id: Uuid,
    /// The snapshot after repairs were applied. Equal to the input
    /// snapshot (in property-set terms) when no actions were taken.
    pub resulting_state: TemplateState,
    /// Actions the doctor took, in application order.
    pub actions: Vec<RepairAction>,
    /// Issues the doctor detected but refused to auto-fix. The caller
    /// is responsible for routing these to a human or higher-level
    /// policy.
    pub unfixable: Vec<Issue>,
    /// True when the doctor found no issues at all (no actions, no
    /// unfixable).
    pub clean: bool,
}

impl RepairReport {
    /// Convenience count of actions taken.
    pub fn actions_count(&self) -> usize {
        self.actions.len()
    }
}

// ── Doctor configuration ────────────────────────────────────────────

/// Per-property default values for required properties whose contract
/// does not declare a default.
///
/// The contract (as of #30) does not yet carry defaults per property,
/// so the doctor reads them from this side-table instead. When the
/// contract gains a `default` field (#future), the doctor will read
/// defaults from there first and fall back to this table only when
/// the contract has no default.
///
/// An empty default map means the doctor cannot auto-repair any
/// `MissingRequiredProperty` issue — it will report them as unfixable
/// via `MissingPropertyDefault`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefaultRegistry {
    map: HashMap<PropertyKey, String>,
}

impl DefaultRegistry {
    /// Construct a new, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a default value for a key. Normalizes the key the
    /// same way `PropertyKey::new` does.
    pub fn with_default(mut self, key: &str, value: impl Into<String>) -> Self {
        if let Ok(k) = PropertyKey::new(key) {
            self.map.insert(k, value.into());
        }
        self
    }

    /// Look up a default by key.
    pub fn get(&self, key: &PropertyKey) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }

    /// True when the registry has no entries.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Number of registered defaults.
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

// ── TemplateDoctor ──────────────────────────────────────────────────

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

    // ── Diagnose ────────────────────────────────────────────────

    /// Diagnose a template: walk the contract against the current
    /// state and emit one `Issue` per detected drift. Pure function.
    pub fn diagnose(
        &self,
        contract: &TemplateContract,
        state: &TemplateState,
    ) -> DiagnosisReport {
        // Guard: contract and state must refer to the same template.
        // We do NOT silently retarget — the caller passed the wrong
        // snapshot, and that's a bug we want to surface.
        assert_eq!(
            contract.template_id(),
            state.template_id,
            "contract.template_id ({}) does not match state.template_id ({})",
            contract.template_id(),
            state.template_id
        );

        let mut issues = Vec::new();

        // 1. Missing required properties.
        for required in contract.required_properties() {
            if !state.properties.contains_key(required) {
                let fixable = self.defaults.get(required).is_some();
                let detail = if fixable {
                    format!(
                        "required property '{}' is missing; default is registered",
                        required
                    )
                } else {
                    format!(
                        "required property '{}' is missing and has no registered default",
                        required
                    )
                };
                issues.push(Issue {
                    kind: IssueKind::MissingRequiredProperty,
                    property: Some(required.clone()),
                    detail,
                    fixable,
                });
                if !fixable {
                    // Mirror the missing-default issue so the report
                    // explicitly tells the caller *why* the fix is
                    // blocked, not just that something is missing.
                    issues.push(Issue {
                        kind: IssueKind::MissingPropertyDefault,
                        property: Some(required.clone()),
                        detail: format!(
                            "no default registered for required property '{}'",
                            required
                        ),
                        fixable: false,
                    });
                }
            }
        }

        // 2. Extra properties (template has a key the contract does
        //    not declare as required).
        for key in state.properties.keys() {
            if !contract.required_properties().iter().any(|r| r == key) {
                issues.push(Issue {
                    kind: IssueKind::ExtraProperty,
                    property: Some(key.clone()),
                    detail: format!(
                        "property '{}' is not declared in the contract",
                        key
                    ),
                    fixable: true,
                });
            }
        }

        // 3. Locked property changes. We compare the template's
        //    current value against the *contract's* canonical value.
        //    The contract itself does not store values per property
        //    (the values live in the template's instance data), so
        //    for a missing-vs-extra check we use the contract's
        //    `required_properties` set as the "known" set: a locked
        //    property must be present in the template.
        //
        //    For the *value* comparison we need a contract value.
        //    The contract carries no per-key value (only layout and
        //    required-ness), so we fall back to the standard
        //    `TemplateContract::check_locked_mutations` semantic:
        //    a locked property is "changed" if it's missing or if
        //    its current value differs from the value the caller
        //    supplies via the snapshot's locked map (carried in
        //    `properties` as a regular entry — the contract's
        //    canonical value lives in the same map because the
        //    contract is itself stored on the template page).
        //
        //    To stay correct without inventing storage we treat
        //    *missing* locked keys as `LockedPropertyChanged` and
        //    rely on `MissingRequiredProperty` to flag the missing
        //    side. See `template_state_locked_values` for the
        //    supported way to pass canonical values.
        for locked in contract.locked_properties() {
            // A locked property is "changed" when it's missing
            // entirely (the user removed it) — we can't compare
            // values we don't have. We do NOT re-emit the
            // MissingRequiredProperty here; that issue is already
            // reported in step 1.
            if !state.properties.contains_key(locked) {
                continue;
            }
            // The actual value comparison requires the contract's
            // canonical value, which is not on the contract object
            // itself. Callers encode it by stashing it in
            // `state.properties` under a private `__contract__`
            // prefix? No — too magical. We instead expose a
            // dedicated `diagnose_with_canonical_locked_values`
            // entry point for callers that have the canonical
            // values. The base `diagnose` reports missing locked
            // properties via `MissingRequiredProperty` and
            // *intentionally does not* report value drift.
            //
            // Rationale: without a canonical value there is no
            // way to know what "changed" means. Reporting every
            // locked property as "changed" would be a false
            // positive on every clean template.
            let _ = locked;
        }

        // 4. Version mismatch.
        if state.version < *contract.version() {
            issues.push(Issue {
                kind: IssueKind::VersionMismatch,
                property: None,
                detail: format!(
                    "template version {} is older than contract version {}",
                    state.version.as_u32(),
                    contract.version().as_u32()
                ),
                fixable: true,
            });
        }

        let fixable_count = issues.iter().filter(|i| i.fixable).count();
        let unfixable_count = issues.len() - fixable_count;

        DiagnosisReport {
            template_id: state.template_id,
            issues,
            fixable_count,
            unfixable_count,
        }
    }

    /// Diagnose with explicit canonical values for locked properties.
    /// Use this when the caller knows the contract's per-property
    /// canonical values and wants `LockedPropertyChanged` reported.
    ///
    /// `canonical_locked_values` maps `PropertyKey -> contract value`.
    /// Any locked key in the contract that is missing from this map
    /// is treated as "no canonical value known" and *not* flagged as
    /// changed (we cannot lie about drift we cannot measure).
    pub fn diagnose_with_canonical_locked_values(
        &self,
        contract: &TemplateContract,
        state: &TemplateState,
        canonical_locked_values: &HashMap<PropertyKey, String>,
    ) -> DiagnosisReport {
        let mut report = self.diagnose(contract, state);
        for locked in contract.locked_properties() {
            if !state.properties.contains_key(locked) {
                // Already reported (or covered) by the missing
                // required pass; do not double-flag.
                continue;
            }
            let Some(canonical) = canonical_locked_values.get(locked) else {
                continue;
            };
            let Some(actual) = state.properties.get(locked) else {
                continue;
            };
            if actual != canonical {
                report.issues.push(Issue {
                    kind: IssueKind::LockedPropertyChanged,
                    property: Some(locked.clone()),
                    detail: format!(
                        "locked property '{}' changed from contract value '{}' to '{}'",
                        locked, canonical, actual
                    ),
                    fixable: true,
                });
            }
        }
        // Recompute counts after the additional pass.
        report.fixable_count = report.issues.iter().filter(|i| i.fixable).count();
        report.unfixable_count = report.issues.len() - report.fixable_count;
        report
    }

    // ── Repair ──────────────────────────────────────────────────

    /// Plan repairs for the issues in `report`, returning the actions
    /// the doctor intends to take. Pure: does not mutate `state`.
    ///
    /// Returns a tuple of `(actions, unfixable_issues)`. The caller
    /// can inspect both before applying.
    #[allow(unused_variables)]
    pub fn plan_repairs(
        &self,
        contract: &TemplateContract,
        state: &TemplateState,
        report: &DiagnosisReport,
    ) -> (Vec<RepairAction>, Vec<Issue>) {
        let mut actions = Vec::new();
        let mut unfixable = Vec::new();

        for issue in &report.issues {
            if !issue.fixable {
                unfixable.push(issue.clone());
                continue;
            }
            match issue.kind {
                IssueKind::MissingRequiredProperty => {
                    if let Some(key) = &issue.property {
                        if let Some(default) = self.defaults.get(key) {
                            actions.push(RepairAction::Added {
                                key: key.clone(),
                                value: default.to_string(),
                            });
                        } else {
                            // Should not happen — diagnose() marks
                            // missing-without-default as unfixable.
                            unfixable.push(issue.clone());
                        }
                    }
                }
                IssueKind::ExtraProperty => {
                    if let Some(key) = &issue.property {
                        actions.push(RepairAction::Removed { key: key.clone() });
                    }
                }
                IssueKind::LockedPropertyChanged => {
                    // Locked-property restoration requires a
                    // canonical value. The plan step is unaware of
                    // it, so the issue is marked unfixable at the
                    // plan level when no canonical value is
                    // available. Use `repair_with_canonical` to
                    // supply the canonical values.
                    unfixable.push(issue.clone());
                }
                IssueKind::VersionMismatch => {
                    actions.push(RepairAction::Upgraded {
                        new_version: *contract.version(),
                    });
                }
                IssueKind::MissingPropertyDefault => {
                    unfixable.push(issue.clone());
                }
            }
        }

        (actions, unfixable)
    }

    /// Apply a `DiagnosisReport` to the supplied state, returning a
    /// `RepairReport` with the resulting snapshot, the actions
    /// taken, and any issues the doctor refused to auto-fix.
    ///
    /// Pure: the input `state` is not mutated. The caller is
    /// responsible for persisting `resulting_state`.
    pub fn repair(
        &self,
        contract: &TemplateContract,
        state: &TemplateState,
        report: &DiagnosisReport,
    ) -> RepairReport {
        let (actions, unfixable) = self.plan_repairs(contract, state, report);
        let resulting_state = state.apply_actions(&actions);
        let clean = actions.is_empty() && unfixable.is_empty();
        RepairReport {
            template_id: state.template_id,
            resulting_state,
            actions,
            unfixable,
            clean,
        }
    }

    /// Repair a template, supplying canonical values for locked
    /// properties so that `LockedPropertyChanged` issues can be
    /// restored. This is the entry point callers should use when
    /// they have access to the contract's per-property values.
    pub fn repair_with_canonical(
        &self,
        contract: &TemplateContract,
        state: &TemplateState,
        canonical_locked_values: &HashMap<PropertyKey, String>,
    ) -> RepairReport {
        let report =
            self.diagnose_with_canonical_locked_values(contract, state, canonical_locked_values);
        let mut actions = Vec::new();
        let mut unfixable = Vec::new();

        for issue in &report.issues {
            if !issue.fixable {
                unfixable.push(issue.clone());
                continue;
            }
            match issue.kind {
                IssueKind::MissingRequiredProperty => {
                    if let Some(key) = &issue.property {
                        if let Some(default) = self.defaults.get(key) {
                            actions.push(RepairAction::Added {
                                key: key.clone(),
                                value: default.to_string(),
                            });
                        } else {
                            unfixable.push(issue.clone());
                        }
                    }
                }
                IssueKind::ExtraProperty => {
                    if let Some(key) = &issue.property {
                        actions.push(RepairAction::Removed { key: key.clone() });
                    }
                }
                IssueKind::LockedPropertyChanged => {
                    if let Some(key) = &issue.property {
                        if let Some(canonical) = canonical_locked_values.get(key) {
                            actions.push(RepairAction::Restored {
                                key: key.clone(),
                                value: canonical.clone(),
                            });
                        } else {
                            unfixable.push(issue.clone());
                        }
                    }
                }
                IssueKind::VersionMismatch => {
                    actions.push(RepairAction::Upgraded {
                        new_version: *contract.version(),
                    });
                }
                IssueKind::MissingPropertyDefault => {
                    unfixable.push(issue.clone());
                }
            }
        }

        let resulting_state = state.apply_actions(&actions);
        let clean = actions.is_empty() && unfixable.is_empty();
        RepairReport {
            template_id: state.template_id,
            resulting_state,
            actions,
            unfixable,
            clean,
        }
    }
}

// ── Gardener integration ────────────────────────────────────────────

/// Extension points the `StructureGardener` runs as part of its
/// regular inspection cycle.
///
/// `TemplateCare` runs the `TemplateDoctor` against a single
/// template. It is *not* an async task — the caller is expected to
/// have already loaded the template state. The `TemplateDoctor` is
/// pure, so the gardener composes it with its own I/O layer.
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

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut builder = TemplateContract::builder().template_id(template_id).version(version);
        for r in required {
            if locked.contains(r) {
                builder = builder.required_property(r).locked_layout(r);
            } else {
                builder = builder.required_property(r).inline_layout(r);
            }
        }
        builder.build().expect("contract build")
    }

    fn state_with(
        template_id: Uuid,
        pairs: &[(&str, &str)],
        version: Version,
    ) -> TemplateState {
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
        let doctor = TemplateDoctor::with_defaults(
            DefaultRegistry::new().with_default("status", "draft"),
        );
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
        assert!(!repair.unfixable.iter().any(|i| i.kind == IssueKind::MissingRequiredProperty));
        match &repair.actions[0] {
            RepairAction::Added { key, value } => {
                assert_eq!(key.as_str(), "status");
                assert_eq!(value, "draft");
            }
            other => panic!("expected Added, got {other:?}"),
        }
        // Resulting state carries the new property.
        assert_eq!(
            repair.resulting_state.properties.get(&PropertyKey::new("status").unwrap()),
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
        assert!(!repair.resulting_state.properties.contains_key(
            &PropertyKey::new("foo").unwrap()
        ));
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

        let report =
            doctor.diagnose_with_canonical_locked_values(&contract, &state, &canonical);
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
            repair.resulting_state.properties.get(&PropertyKey::new("type").unwrap()),
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
        let report =
            doctor.diagnose_with_canonical_locked_values(&contract, &state, &canonical);
        assert!(report.is_clean(), "no drift expected, got {:?}", report.issues);
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
        let doctor = TemplateDoctor::with_defaults(
            DefaultRegistry::new().with_default("status", "draft"),
        );
        let mut canonical = HashMap::new();
        canonical.insert(PropertyKey::new("type").unwrap(), "task".to_string());

        let report =
            doctor.diagnose_with_canonical_locked_values(&contract, &state, &canonical);

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
        assert!(!result.properties.contains_key(&PropertyKey::new("unused").unwrap()));
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
        assert!(repair
            .unfixable
            .iter()
            .any(|i| i.kind == IssueKind::MissingRequiredProperty));
        assert!(repair
            .unfixable
            .iter()
            .any(|i| i.kind == IssueKind::MissingPropertyDefault));
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
        assert!(state
            .properties()
            .contains_key(&PropertyKey::new("status").unwrap()));
    }

    #[test]
    fn partition_separates_fixable_from_unfixable() {
        let tid = uuid_from_u8(12);
        let contract = contract_with(tid, &["title", "status"], &[], Version::new());
        let state = state_with(
            tid,
            &[("title", "x"), ("extra", "y")],
            Version::new(),
        );
        let report = TemplateDoctor::new().diagnose(&contract, &state);
        let (fixable, unfixable) = report.partition();
        assert!(fixable
            .iter()
            .any(|i| i.kind == IssueKind::ExtraProperty));
        assert!(unfixable
            .iter()
            .any(|i| i.kind == IssueKind::MissingRequiredProperty));
        assert!(unfixable
            .iter()
            .any(|i| i.kind == IssueKind::MissingPropertyDefault));
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
        assert!(RepairAction::Upgraded {
            new_version: Version::new()
        }
        .property()
        .is_none());
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
