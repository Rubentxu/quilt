//! Heading projection contract.
//
// Matches blocks where `block-role:: heading` AND `heading-level::` is 1, 2, or 3.
// Produces a HeadingAnchor decoration with weight based on level.

use quilt_domain::entities::{Block, PropertyKey};
use quilt_domain::projection::contract::{ProjectionContract, ProjectionContractId};
use quilt_domain::projection::projection_trait::{Projection, ProjectionContext};
use quilt_domain::projection::view::{Decoration, DecorationKind, ProjectionViewDelta};
use quilt_domain::value_objects::PropertyValue;

/// HeadingProjection — produces a heading-anchor decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadingProjection;

impl HeadingProjection {
    /// Construct a new `HeadingProjection`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Weight based on heading level — h1 is most prominent.
    fn level_weight(level: i64) -> u8 {
        match level {
            1 => 100,
            2 => 80,
            3 => 60,
            _ => 40,
        }
    }
}

impl Projection for HeadingProjection {
    fn contract_id(&self) -> ProjectionContractId {
        ProjectionContractId::new("heading").expect("'heading' is a valid contract ID")
    }

    fn apply(&self, block: &Block, _ctx: &ProjectionContext) -> ProjectionViewDelta {
        let level_value = block
            .properties
            .get("heading-level")
            .cloned()
            .unwrap_or_else(|| PropertyValue::integer(1));

        let weight = match level_value {
            PropertyValue::Integer(n) => Self::level_weight(n),
            _ => 60,
        };

        let decoration = Decoration {
            kind: DecorationKind::HeadingAnchor,
            target: PropertyKey::new("heading-level").unwrap(),
            value: level_value,
            weight,
        };

        let view_properties = vec![(
            PropertyKey::new("projection").unwrap(),
            PropertyValue::string("heading"),
        )];

        ProjectionViewDelta {
            decorations: vec![decoration],
            view_properties,
            conflicts: vec![],
        }
    }
}

/// The V1 heading contract.
#[must_use]
pub fn heading_contract() -> ProjectionContract {
    ProjectionContract::new(ProjectionContractId::new("heading").unwrap())
        .with_priority(150)
        .with_predicates(vec![
            quilt_domain::projection::PropertyPredicate::Equals {
                key: PropertyKey::new("block-role").unwrap(),
                value: PropertyValue::string("heading"),
            },
            // heading-level must be 1, 2, or 3
            quilt_domain::projection::PropertyPredicate::IsOneOf {
                key: PropertyKey::new("heading-level").unwrap(),
                values: vec![
                    PropertyValue::integer(1),
                    PropertyValue::integer(2),
                    PropertyValue::integer(3),
                ],
            },
        ])
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
            content: "Test heading".into(),
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
    fn heading_projection_contract_id() {
        assert_eq!(HeadingProjection::new().contract_id().as_str(), "heading");
    }

    #[test]
    fn heading_apply_returns_anchor_decoration() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("heading"));
            p.insert("heading-level".into(), PropertyValue::integer(1));
            p
        });
        let delta = HeadingProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert_eq!(delta.decorations.len(), 1);
        assert_eq!(delta.decorations[0].kind, DecorationKind::HeadingAnchor);
        assert_eq!(delta.decorations[0].weight, 100); // h1 → weight 100
    }

    #[test]
    fn heading_apply_includes_projection_property() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("heading"));
            p.insert("heading-level".into(), PropertyValue::integer(2));
            p
        });
        let delta = HeadingProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert!(delta.view_properties.iter().any(|(k, v)| {
            k.as_str() == "projection" && *v == PropertyValue::string("heading")
        }));
    }

    #[test]
    fn heading_contract_matches_role_heading_with_h1() {
        let contract = heading_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("heading"));
            p.insert("heading-level".into(), PropertyValue::integer(1));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn heading_contract_matches_h2() {
        let contract = heading_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("heading"));
            p.insert("heading-level".into(), PropertyValue::integer(2));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn heading_contract_matches_h3() {
        let contract = heading_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("heading"));
            p.insert("heading-level".into(), PropertyValue::integer(3));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn heading_contract_rejects_h4() {
        let contract = heading_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("heading"));
            p.insert("heading-level".into(), PropertyValue::integer(4));
            p
        });
        assert!(!contract.matches_block(&block));
    }

    #[test]
    fn heading_contract_rejects_non_heading_role() {
        let contract = heading_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("paragraph"));
            p.insert("heading-level".into(), PropertyValue::integer(1));
            p
        });
        assert!(!contract.matches_block(&block));
    }
}
