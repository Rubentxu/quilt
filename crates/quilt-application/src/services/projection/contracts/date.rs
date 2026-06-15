//! Date projection contract.
//
// Matches blocks where `scheduled::` OR `deadline::` exists.
// Produces a DateIndicator decoration — weight is higher for deadline.

use quilt_domain::entities::{Block, PropertyKey};
use quilt_domain::projection::contract::{ProjectionContract, ProjectionContractId};
use quilt_domain::projection::projection_trait::{Projection, ProjectionContext};
use quilt_domain::projection::view::{Decoration, DecorationKind, ProjectionViewDelta};
use quilt_domain::value_objects::PropertyValue;

/// DateProjection — produces a date-indicator decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct DateProjection;

impl DateProjection {
    /// Construct a new `DateProjection`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Projection for DateProjection {
    fn contract_id(&self) -> ProjectionContractId {
        ProjectionContractId::new("date").expect("'date' is a valid contract ID")
    }

    fn apply(&self, block: &Block, _ctx: &ProjectionContext) -> ProjectionViewDelta {
        // Prefer deadline over scheduled — higher weight
        let (target_key, date_value, weight) =
            if let Some(v) = block.properties.get("deadline").cloned() {
                ("deadline".to_string(), v, 95u8)
            } else if let Some(v) = block.properties.get("scheduled").cloned() {
                ("scheduled".to_string(), v, 75u8)
            } else {
                // Should not be reached — contract ensures one of these is set
                return ProjectionViewDelta::default();
            };

        let decoration = Decoration {
            kind: DecorationKind::DateIndicator,
            target: PropertyKey::new(&target_key).unwrap(),
            value: date_value,
            weight,
        };

        let view_properties = vec![(
            PropertyKey::new("projection").unwrap(),
            PropertyValue::string("date"),
        )];

        ProjectionViewDelta {
            decorations: vec![decoration],
            view_properties,
            conflicts: vec![],
        }
    }
}

/// The V1 date contract — matches when `scheduled::` OR `deadline::` is set.
#[must_use]
pub fn date_contract() -> ProjectionContract {
    // OR via combinator: IsSet(scheduled) OR IsSet(deadline)
    let has_scheduled = quilt_domain::projection::PropertyPredicate::IsSet {
        key: PropertyKey::new("scheduled").unwrap(),
    };
    let has_deadline = quilt_domain::projection::PropertyPredicate::IsSet {
        key: PropertyKey::new("deadline").unwrap(),
    };
    let or_date = quilt_domain::projection::PropertyPredicate::Or(
        Box::new(has_scheduled),
        Box::new(has_deadline),
    );

    ProjectionContract::new(ProjectionContractId::new("date").unwrap())
        .with_priority(250)
        .with_predicates(vec![or_date])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
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
            content: "Test date".into(),
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
    fn date_projection_contract_id() {
        assert_eq!(DateProjection::new().contract_id().as_str(), "date");
    }

    #[test]
    fn date_apply_returns_indicator_decoration_for_deadline() {
        let dt = Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("deadline".into(), PropertyValue::date(dt));
            p
        });
        let delta = DateProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert_eq!(delta.decorations.len(), 1);
        assert_eq!(delta.decorations[0].kind, DecorationKind::DateIndicator);
        assert_eq!(delta.decorations[0].weight, 95); // deadline → weight 95
    }

    #[test]
    fn date_apply_returns_indicator_decoration_for_scheduled() {
        let dt = Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("scheduled".into(), PropertyValue::date(dt));
            p
        });
        let delta = DateProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert_eq!(delta.decorations.len(), 1);
        assert_eq!(delta.decorations[0].kind, DecorationKind::DateIndicator);
        assert_eq!(delta.decorations[0].weight, 75); // scheduled → weight 75
    }

    #[test]
    fn date_apply_includes_projection_property() {
        let dt = Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("scheduled".into(), PropertyValue::date(dt));
            p
        });
        let delta = DateProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert!(delta.view_properties.iter().any(|(k, v)| {
            k.as_str() == "projection" && *v == PropertyValue::string("date")
        }));
    }

    #[test]
    fn date_contract_matches_deadline() {
        let contract = date_contract();
        let dt = Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("deadline".into(), PropertyValue::date(dt));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn date_contract_matches_scheduled() {
        let contract = date_contract();
        let dt = Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("scheduled".into(), PropertyValue::date(dt));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn date_contract_rejects_when_neither_is_set() {
        let contract = date_contract();
        let block = make_block(HashMap::new());
        assert!(!contract.matches_block(&block));
    }
}
