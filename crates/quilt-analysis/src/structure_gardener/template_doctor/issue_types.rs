//! Issue types, repair actions, snapshots, and report data structures
//! for the Template Doctor.
//!
//! This module is **data only**: it defines the shapes the doctor
//! produces and consumes but holds no logic. `diagnosis.rs` and
//! `repair.rs` operate on these types; the orchestrating
//! `TemplateDoctor` lives in `lib.rs`.

use quilt_domain::entities::{PropertyKey, Version};
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
    pub fn new(template_id: Uuid, properties: HashMap<String, String>, version: Version) -> Self {
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
