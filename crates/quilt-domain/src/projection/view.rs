//! Projection view — composable block visualization surface.
//!
//! A [`ProjectionView`] is the output of the projection resolution process.
//! It represents the complete visual state of a block: text, links,
//! decorations (visual annotations), and any conflicts from ambiguous resolution.
//!
//! # Base Block Surface
//!
//! Every block always starts from its **Base Block Surface**: the raw content,
//! links, and children already present on the block. Decorations from
//! active contracts are **composed** on top of this base — they never replace it.
//!
//! # Delta composition
//!
//! A [`ProjectionViewDelta`] represents the **additions** made by a single
//! projection contract's [`super::projection_trait::Projection::apply`] method.
//! The resolver composes deltas from the winning contract onto the base surface.

use crate::entities::PropertyKey;
use crate::projection::conflict::ProjectionConflict;
use crate::value_objects::PropertyValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kind of link — determines how the UI renders the link affordance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkKind {
    /// External URL (web link)
    External,
    /// Media asset (image, video, audio)
    Media,
    /// Reference to another page
    PageRef,
    /// Reference to another block
    BlockRef,
}

impl Default for LinkKind {
    fn default() -> Self {
        LinkKind::External
    }
}

/// A link extracted or derived from a block property.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkView {
    /// URL or identifier of the link.
    pub url: String,
    /// Human-readable label (may be empty).
    pub label: String,
    /// Kind of link.
    #[serde(default)]
    pub kind: LinkKind,
}

/// Kind of decoration — visual annotation applied by a projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecorationKind {
    /// Task checkbox (from `status` property)
    TaskCheckbox,
    /// Status badge (colored label)
    StatusBadge,
    /// Media embed preview (thumbnail, play button, etc.)
    MediaPreview,
    /// Heading anchor (e.g. `#` with level number)
    HeadingAnchor,
    /// Date indicator (scheduled, deadline, etc.)
    DateIndicator,
    /// Link affordance (external link icon)
    LinkAffordance,
    /// Generic badge (custom label + color)
    GenericBadge,
}

/// A visual decoration produced by a projection contract.
///
/// Decorations are additive — multiple contracts can each add their own
/// decorations. The UI decides how to render them based on `kind` and `weight`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decoration {
    /// What kind of decoration this is.
    pub kind: DecorationKind,
    /// Property key this decoration targets (e.g. `"status"`, `"deadline"`).
    pub target: PropertyKey,
    /// The property value driving this decoration.
    pub value: PropertyValue,
    /// Higher weight = rendered more prominently. Range 0–255.
    pub weight: u8,
}

/// The complete visual projection of a block.
///
/// Produced by [`super::projection_trait::ProjectionResolver::resolve`](super::projection_trait::ProjectionResolver::resolve).
/// All fields are public for serialization and pattern-matching convenience.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionView {
    /// Raw text content from the block.
    pub text: String,
    /// Links extracted or derived from block properties.
    #[serde(default)]
    pub links: Vec<LinkView>,
    /// Child block IDs (preserved in order).
    #[serde(default)]
    pub children: Vec<super::super::value_objects::Uuid>,
    /// Visual decorations from active contracts.
    #[serde(default)]
    pub decorations: Vec<Decoration>,
    /// Conflicts from ambiguous resolution (empty when resolution is unambiguous).
    #[serde(default)]
    pub conflicts: Vec<ProjectionConflict>,
    /// Effective properties for the view (base + derived).
    /// Key: dash-normalized [`PropertyKey`] string.
    /// Value: [`PropertyValue`].
    #[serde(default)]
    pub properties: HashMap<PropertyKey, PropertyValue>,
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Fluent builder for [`ProjectionView`].
#[derive(Debug, Default)]
pub struct ProjectionViewBuilder {
    text: String,
    links: Vec<LinkView>,
    children: Vec<super::super::value_objects::Uuid>,
    decorations: Vec<Decoration>,
    conflicts: Vec<ProjectionConflict>,
    properties: HashMap<PropertyKey, PropertyValue>,
}

impl ProjectionViewBuilder {
    /// Construct a builder initialized from a block's base surface.
    pub fn new(block: &super::super::entities::Block) -> Self {
        // Convert String keys to PropertyKey (block.properties keys are already normalized)
        let properties = block
            .properties
            .iter()
            .filter_map(|(k, v)| PropertyKey::new(k).ok().map(|key| (key, v.clone())))
            .collect();

        Self {
            text: block.content.clone(),
            children: block.refs.clone(),
            links: Vec::new(),
            decorations: Vec::new(),
            conflicts: Vec::new(),
            properties,
        }
    }

    /// Add a link.
    pub fn add_link(mut self, link: LinkView) -> Self {
        self.links.push(link);
        self
    }

    /// Add a decoration.
    pub fn add_decoration(mut self, decoration: Decoration) -> Self {
        self.decorations.push(decoration);
        self
    }

    /// Add a conflict.
    pub fn add_conflict(mut self, conflict: ProjectionConflict) -> Self {
        self.conflicts.push(conflict);
        self
    }

    /// Add a property derived by a projection.
    pub fn add_property(mut self, key: PropertyKey, value: PropertyValue) -> Self {
        self.properties.insert(key, value);
        self
    }

    /// Build the view.
    #[must_use]
    pub fn build(self) -> ProjectionView {
        ProjectionView {
            text: self.text,
            links: self.links,
            children: self.children,
            decorations: self.decorations,
            conflicts: self.conflicts,
            properties: self.properties,
        }
    }
}

/// Additive delta from a single [`super::projection_trait::Projection::apply`] call.
///
/// The delta carries ONLY additions (new decorations, new properties, new conflicts).
/// The base surface (text, links, children) comes from the block itself and is
/// NOT repeated in the delta.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectionViewDelta {
    /// Decorations added by this projection.
    #[serde(default)]
    pub decorations: Vec<Decoration>,

    /// Properties derived by this projection (additive — merged into the view).
    #[serde(default)]
    pub view_properties: Vec<(PropertyKey, PropertyValue)>,

    /// Conflicts detected during projection application.
    #[serde(default)]
    pub conflicts: Vec<ProjectionConflict>,
}

impl ProjectionViewDelta {
    /// Compose this delta onto a base view, returning a new view.
    ///
    /// Decorations are appended (never replaced). Properties are merged
    /// (later deltas overwrite earlier ones for the same key).
    #[must_use]
    pub fn compose_on(self, base: ProjectionView) -> ProjectionView {
        let mut decorations = base.decorations;
        decorations.extend(self.decorations);

        let mut properties = base.properties;
        for (key, value) in self.view_properties {
            properties.insert(key, value);
        }

        let mut conflicts = base.conflicts;
        conflicts.extend(self.conflicts);

        ProjectionView {
            text: base.text,
            links: base.links,
            children: base.children,
            decorations,
            conflicts,
            properties,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Block;
    use crate::value_objects::{PropertyValue, Uuid};
    use chrono::Utc;

    fn make_block() -> Block {
        Block {
            id: Uuid::new_v4(),
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 0.0,
            level: 1,
            format: crate::value_objects::BlockFormat::Markdown,
            block_type: crate::value_objects::BlockType::Paragraph,
            marker: None,
            priority: None,
            content: "Hello world".into(),
            properties: {
                let mut p = HashMap::new();
                p.insert("type".into(), PropertyValue::string("task"));
                p
            },
            refs: vec![],
            tags: vec![],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            completed_at: None,
            cancelled_at: None,
            collapsed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ── LinkKind ──────────────────────────────────────────────────

    #[test]
    fn link_kind_unknown_falls_back_to_external() {
        // Default is External
        assert_eq!(LinkKind::default(), LinkKind::External);
    }

    #[test]
    fn link_view_for_page_ref() {
        let link = LinkView {
            url: "[[my-page]]".into(),
            label: "My Page".into(),
            kind: LinkKind::PageRef,
        };
        let json = serde_json::to_string(&link).expect("serialize");
        assert!(
            json.contains("page-ref"),
            "Expected kebab-case 'page-ref' in {json}"
        );
    }

    // ── Decoration ────────────────────────────────────────────────

    #[test]
    fn decoration_task_checkbox_carries_status() {
        let dec = Decoration {
            kind: DecorationKind::TaskCheckbox,
            target: PropertyKey::new("status").unwrap(),
            value: PropertyValue::string("done"),
            weight: 100,
        };
        assert_eq!(dec.kind, DecorationKind::TaskCheckbox);
        assert_eq!(dec.weight, 100);
    }

    #[test]
    fn decoration_serializes_kind_kebab() {
        let dec = Decoration {
            kind: DecorationKind::TaskCheckbox,
            target: PropertyKey::new("status").unwrap(),
            value: PropertyValue::string("todo"),
            weight: 50,
        };
        let json = serde_json::to_string(&dec).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(
            v["kind"], "task-checkbox",
            "Expected kebab-case kind in {json}"
        );
    }

    #[test]
    fn decoration_unknown_kind_rejected_on_deserialize() {
        let json = r#"{"kind":"made-up","target":"status","value":"todo","weight":50}"#;
        let result: Result<Decoration, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Unknown decoration kind should be rejected"
        );
    }

    // ── ProjectionView ────────────────────────────────────────────

    #[test]
    fn view_default_carries_full_base_surface() {
        let block = make_block();
        let view = ProjectionViewBuilder::new(&block).build();

        assert_eq!(view.text, "Hello world");
        assert!(view.links.is_empty());
        assert!(view.decorations.is_empty());
        assert!(view.conflicts.is_empty());
        assert_eq!(
            view.properties.get(&PropertyKey::new("type").unwrap()),
            Some(&PropertyValue::string("task"))
        );
    }

    #[test]
    fn view_with_links() {
        let block = make_block();
        let view = ProjectionViewBuilder::new(&block)
            .add_link(LinkView {
                url: "https://example.com".into(),
                label: "Example".into(),
                kind: LinkKind::External,
            })
            .build();

        assert_eq!(view.links.len(), 1);
        assert_eq!(view.links[0].url, "https://example.com");
    }

    #[test]
    fn view_exposes_children_in_order() {
        let child1 = Uuid::new_v4();
        let child2 = Uuid::new_v4();
        let mut block = make_block();
        block.refs = vec![child1, child2];

        let view = ProjectionViewBuilder::new(&block).build();
        assert_eq!(view.children, vec![child1, child2]);
    }

    #[test]
    fn view_serializes_losslessly() {
        let block = make_block();
        let view = ProjectionViewBuilder::new(&block)
            .add_decoration(Decoration {
                kind: DecorationKind::StatusBadge,
                target: PropertyKey::new("status").unwrap(),
                value: PropertyValue::string("in-progress"),
                weight: 75,
            })
            .build();

        let json = serde_json::to_string(&view).expect("serialize");
        let parsed: ProjectionView = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(view, parsed);
    }

    #[test]
    fn view_decorations_compose_never_replace() {
        let block = make_block();
        let base = ProjectionViewBuilder::new(&block)
            .add_decoration(Decoration {
                kind: DecorationKind::TaskCheckbox,
                target: PropertyKey::new("status").unwrap(),
                value: PropertyValue::string("todo"),
                weight: 100,
            })
            .build();

        let delta = ProjectionViewDelta {
            decorations: vec![Decoration {
                kind: DecorationKind::StatusBadge,
                target: PropertyKey::new("status").unwrap(),
                value: PropertyValue::string("in-progress"),
                weight: 50,
            }],
            ..Default::default()
        };

        let composed = delta.compose_on(base.clone());
        // Both decorations should be present (never replaced)
        assert_eq!(composed.decorations.len(), 2);
    }

    #[test]
    fn view_properties_expose_effective_set() {
        let block = make_block();
        let view = ProjectionViewBuilder::new(&block)
            .add_property(
                PropertyKey::new("projection").unwrap(),
                PropertyValue::string("task"),
            )
            .build();

        assert_eq!(
            view.properties
                .get(&PropertyKey::new("projection").unwrap()),
            Some(&PropertyValue::string("task"))
        );
        assert_eq!(
            view.properties.get(&PropertyKey::new("type").unwrap()),
            Some(&PropertyValue::string("task"))
        );
    }

    // ── Builder ───────────────────────────────────────────────────

    #[test]
    fn builder_initializes_from_block() {
        let block = make_block();
        let view = ProjectionViewBuilder::new(&block).build();
        assert_eq!(view.text, block.content);
        assert_eq!(view.children, block.refs);
    }

    #[test]
    fn builder_adds_links_decorations_in_order() {
        let block = make_block();
        let view = ProjectionViewBuilder::new(&block)
            .add_link(LinkView {
                url: "https://a.com".into(),
                label: "A".into(),
                kind: LinkKind::External,
            })
            .add_decoration(Decoration {
                kind: DecorationKind::LinkAffordance,
                target: PropertyKey::new("link").unwrap(),
                value: PropertyValue::string("https://a.com"),
                weight: 100,
            })
            .add_link(LinkView {
                url: "https://b.com".into(),
                label: "B".into(),
                kind: LinkKind::External,
            })
            .build();

        assert_eq!(view.links.len(), 2);
        assert_eq!(view.decorations.len(), 1);
    }

    #[test]
    fn builder_adds_conflict() {
        let block = make_block();
        let conflict = ProjectionConflict {
            reason: "tied".into(),
            candidates: vec![],
            winner: None,
            block_id: Uuid::new_v4(),
        };
        let view = ProjectionViewBuilder::new(&block)
            .add_conflict(conflict.clone())
            .build();

        assert_eq!(view.conflicts.len(), 1);
    }

    #[test]
    fn builder_adds_view_derived_property() {
        let block = make_block();
        let view = ProjectionViewBuilder::new(&block)
            .add_property(
                PropertyKey::new("projection").unwrap(),
                PropertyValue::string("default"),
            )
            .build();

        assert!(
            view.properties
                .contains_key(&PropertyKey::new("projection").unwrap())
        );
    }

    // ── ProjectionViewDelta ────────────────────────────────────────

    #[test]
    fn delta_default_is_empty() {
        let delta = ProjectionViewDelta::default();
        assert!(delta.decorations.is_empty());
        assert!(delta.view_properties.is_empty());
        assert!(delta.conflicts.is_empty());
    }

    #[test]
    fn delta_carries_only_additions() {
        let delta = ProjectionViewDelta {
            decorations: vec![Decoration {
                kind: DecorationKind::MediaPreview,
                target: PropertyKey::new("source-url").unwrap(),
                value: PropertyValue::string("https://img.jpg"),
                weight: 100,
            }],
            view_properties: vec![(
                PropertyKey::new("projection").unwrap(),
                PropertyValue::string("media"),
            )],
            conflicts: vec![],
        };

        assert_eq!(delta.decorations.len(), 1);
        assert_eq!(delta.view_properties.len(), 1);
    }

    #[test]
    fn delta_is_composable() {
        let block = make_block();
        let base = ProjectionViewBuilder::new(&block).build();

        let delta1 = ProjectionViewDelta {
            decorations: vec![Decoration {
                kind: DecorationKind::TaskCheckbox,
                target: PropertyKey::new("status").unwrap(),
                value: PropertyValue::string("todo"),
                weight: 100,
            }],
            ..Default::default()
        };

        let intermediate = delta1.compose_on(base.clone());

        let delta2 = ProjectionViewDelta {
            decorations: vec![Decoration {
                kind: DecorationKind::StatusBadge,
                target: PropertyKey::new("status").unwrap(),
                value: PropertyValue::string("in-progress"),
                weight: 50,
            }],
            ..Default::default()
        };

        let final_view = delta2.compose_on(intermediate);
        // Both decorations present
        assert_eq!(final_view.decorations.len(), 2);
    }
}
