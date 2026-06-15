//! Media projection contract.
//
// Matches blocks where `type:: media` AND `media-type::` is `video` or `image`.
// Produces a MediaPreview decoration.

use quilt_domain::entities::{Block, PropertyKey};
use quilt_domain::projection::contract::{ProjectionContract, ProjectionContractId};
use quilt_domain::projection::projection_trait::{Projection, ProjectionContext};
use quilt_domain::projection::view::{Decoration, DecorationKind, ProjectionViewDelta};
use quilt_domain::value_objects::PropertyValue;

/// MediaProjection — produces a media-preview decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct MediaProjection;

impl MediaProjection {
    /// Construct a new `MediaProjection`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Projection for MediaProjection {
    fn contract_id(&self) -> ProjectionContractId {
        ProjectionContractId::new("media").expect("'media' is a valid contract ID")
    }

    fn apply(&self, block: &Block, _ctx: &ProjectionContext) -> ProjectionViewDelta {
        let media_type = block
            .properties
            .get("media-type")
            .cloned()
            .unwrap_or_else(|| PropertyValue::string("image"));

        let decoration = Decoration {
            kind: DecorationKind::MediaPreview,
            target: PropertyKey::new("media-type").unwrap(),
            value: media_type.clone(),
            weight: 90,
        };

        let view_properties = vec![(
            PropertyKey::new("projection").unwrap(),
            PropertyValue::string("media"),
        )];

        ProjectionViewDelta {
            decorations: vec![decoration],
            view_properties,
            conflicts: vec![],
        }
    }
}

/// The V1 media contract.
#[must_use]
pub fn media_contract() -> ProjectionContract {
    ProjectionContract::new(ProjectionContractId::new("media").unwrap())
        .with_priority(200)
        .with_predicates(vec![
            quilt_domain::projection::PropertyPredicate::Equals {
                key: PropertyKey::new("type").unwrap(),
                value: PropertyValue::string("media"),
            },
            quilt_domain::projection::PropertyPredicate::IsOneOf {
                key: PropertyKey::new("media-type").unwrap(),
                values: vec![
                    PropertyValue::string("video"),
                    PropertyValue::string("image"),
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
            content: "Test media".into(),
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
    fn media_projection_contract_id() {
        assert_eq!(MediaProjection::new().contract_id().as_str(), "media");
    }

    #[test]
    fn media_apply_returns_preview_decoration() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("media"));
            p.insert("media-type".into(), PropertyValue::string("video"));
            p
        });
        let delta = MediaProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert_eq!(delta.decorations.len(), 1);
        assert_eq!(delta.decorations[0].kind, DecorationKind::MediaPreview);
        assert_eq!(delta.decorations[0].weight, 90);
    }

    #[test]
    fn media_apply_includes_projection_property() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("media"));
            p.insert("media-type".into(), PropertyValue::string("image"));
            p
        });
        let delta = MediaProjection::new().apply(&block, &ProjectionContext::page(Utc::now()));

        assert!(delta.view_properties.iter().any(|(k, v)| {
            k.as_str() == "projection" && *v == PropertyValue::string("media")
        }));
    }

    #[test]
    fn media_contract_matches_type_media_with_video() {
        let contract = media_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("media"));
            p.insert("media-type".into(), PropertyValue::string("video"));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn media_contract_matches_type_media_with_image() {
        let contract = media_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("media"));
            p.insert("media-type".into(), PropertyValue::string("image"));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn media_contract_rejects_type_media_with_audio() {
        let contract = media_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("media"));
            p.insert("media-type".into(), PropertyValue::string("audio"));
            p
        });
        assert!(!contract.matches_block(&block));
    }

    #[test]
    fn media_contract_rejects_non_media() {
        let contract = media_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("media-type".into(), PropertyValue::string("video"));
            p
        });
        assert!(!contract.matches_block(&block));
    }
}
