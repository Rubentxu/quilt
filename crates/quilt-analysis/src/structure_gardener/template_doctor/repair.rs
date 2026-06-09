//! Repair logic for the Template Doctor.
//!
//! `TemplateDoctor::plan_repairs` produces a list of `RepairAction`s
//! from a `DiagnosisReport`. `repair` and `repair_with_canonical`
//! apply those actions (purely) and produce a `RepairReport`.

use super::issue_types::{
    DiagnosisReport, Issue, IssueKind, RepairAction, RepairReport, TemplateState,
};
use super::TemplateDoctor;
use quilt_domain::entities::{PropertyKey, TemplateContract};
use std::collections::HashMap;

impl TemplateDoctor {
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
                        if let Some(default) = self.defaults().get(key) {
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
        let report = self.diagnose_with_canonical_locked_values(contract, state, canonical_locked_values);
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
                        if let Some(default) = self.defaults().get(key) {
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
