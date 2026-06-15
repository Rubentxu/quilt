//! Link projection contract.
//
// Matches blocks where `link::` property exists.
// Produces a LinkAffordance decoration.

use quilt_domain::entities::{Block, PropertyKey};
use quilt_domain::projection::contract::{ProjectionContract, ProjectionContractId};
use quilt_domain::projection::projection_trait::{Projection, ProjectionContext};
use quilt_domain::projection::view::{Decoration, DecorationKind, ProjectionViewDelta};
use quilt_domain::value_objects::PropertyValue;

/// LinkProjection — produces a link-affordance decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkProjection;

impl LinkProjection {
    /// Construct a new `LinkProjection`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Projection for LinkProjection {
    fn contract_id(&self) -> ProjectionContractId {
        ProjectionContractId::new("link").expect("'link' is a valid contract ID")
    }

    fn apply(&self, block: &Block, _ctx: &ProjectionContext) -> ProjectionViewDelta {
        let link_value = block
            .properties
            .get("link")
            .cloned()
            .unwrap_or_else(|| PropertyValue::string(""));

        let decoration = Decoration {
            kind: DecorationKind::LinkAffordance,
            target: PropertyKey::new("link").unwrap(),
            value: link_value,
            weight: 70,
        };

        let view_properties = vec![(
            PropertyKey::new("projection").unwrap(),
            PropertyValue::string("link"),
        )];

        ProjectionViewDelta {
            decorations: vec![decoration],
            view_properties,
            conflicts: vec![],
        }
    }
}

/// The V1 link contract.
#[must_use]
pub fn link_contract() -> ProjectionContract {
    ProjectionContract::new(ProjectionContractId::new("link").unwrap())
        .with_priority(300)
        .with_predicates(vec![quilt_domain::projection::PropertyPredicate::IsSet {
            key: PropertyKey::new("link").unwrap(),
        }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_block(props: HashMap<String, PropertyValue>) -> Block {
        Block {
            id: quilt_domain::value_objects::Uuid::new_v4(),
            page_id: quilt_domain::value_objects::Uuid::new_v4(),
            parent_id: None,
            order: 0.0,
            level: 1,
            format: quilt_domain::value_objects::BlockFormat::Markdown,
            block_type: quilt_domain::value_objects::BlockType::Paragraph,
            marker: None,
            priority: None,
            content: "Test link".into(),
            properties: props,
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

    #[test]
    fn link_projection_contract_id() {
        assert_eq!(LinkProjection::new().contract_id().as_str(), "link");
    }

    #[test]
    fn link_apply_returns_affordance_decoration() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("link".into(), PropertyValue::string("https://example.com"));
            p
        });
        let delta = LinkProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert_eq!(delta.decorations.len(), 1);
        assert_eq!(delta.decorations[0].kind, DecorationKind::LinkAffordance);
        assert_eq!(delta.decorations[0].weight, 70);
    }

    #[test]
    fn link_apply_includes_projection_property() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("link".into(), PropertyValue::string("https://example.com"));
            p
        });
        let delta = LinkProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert!(delta.view_properties.iter().any(|(k, v)| {
            k.as_str() == "projection" && *v == PropertyValue::string("link")
        }));
    }

    #[test]
    fn link_contract_matches_when_link_is_set() {
        let contract = link_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("link".into(), PropertyValue::string("https://example.com"));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn link_contract_matches_with_empty_string() {
        // IsSet checks presence, not emptiness — empty string IS set
        let contract = link_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("link".into(), PropertyValue::string(""));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn link_contract_rejects_when_link_is_absent() {
        let contract = link_contract();
        let block = make_block(HashMap::new());
        assert!(!contract.matches_block(&block));
    }
}
