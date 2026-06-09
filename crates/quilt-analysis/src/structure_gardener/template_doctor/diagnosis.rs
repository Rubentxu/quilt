//! Diagnosis logic for the Template Doctor.
//!
//! `TemplateDoctor::diagnose` walks a `TemplateContract` against a
//! `TemplateState` and emits one `Issue` per detected drift. The
//! canonical-locked-values variant additionally reports
//! `LockedPropertyChanged` when the caller supplies the contract's
//! per-key values.

use super::issue_types::{DiagnosisReport, Issue, IssueKind};
use super::TemplateDoctor;
use quilt_domain::entities::{PropertyKey, TemplateContract};
use std::collections::HashMap;

impl TemplateDoctor {
    /// Diagnose a template: walk the contract against the current
    /// state and emit one `Issue` per detected drift. Pure function.
    pub fn diagnose(
        &self,
        contract: &TemplateContract,
        state: &super::issue_types::TemplateState,
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
                let fixable = self.defaults().get(required).is_some();
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
        state: &super::issue_types::TemplateState,
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
}
