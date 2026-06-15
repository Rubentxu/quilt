//! Task projection contract.
//
// Matches blocks where `type:: task` AND `status::` is set.
// Produces a TaskCheckbox decoration with weight based on status.

use quilt_domain::entities::{Block, PropertyKey};
use quilt_domain::projection::contract::{ProjectionContract, ProjectionContractId};
use quilt_domain::projection::projection_trait::{Projection, ProjectionContext};
use quilt_domain::projection::view::{Decoration, DecorationKind, ProjectionViewDelta};
use quilt_domain::value_objects::PropertyValue;

/// TaskProjection — produces a task-checkbox decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskProjection;

impl TaskProjection {
    /// Construct a new `TaskProjection`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Status-to-weight mapping — higher weight for more prominent states.
    fn status_weight(status: &PropertyValue) -> u8 {
        match status {
            PropertyValue::String(s) => match s.as_str() {
                "done" => 100,
                "cancelled" => 80,
                "in-progress" => 60,
                "waiting" => 40,
                "todo" => 20,
                _ => 10,
            },
            _ => 10,
        }
    }
}

impl Projection for TaskProjection {
    fn contract_id(&self) -> ProjectionContractId {
        ProjectionContractId::new("task").expect("'task' is a valid contract ID")
    }

    fn apply(&self, block: &Block, _ctx: &ProjectionContext) -> ProjectionViewDelta {
        // Retrieve the status value to drive the decoration
        let status_value = block
            .properties
            .get("status")
            .cloned()
            .unwrap_or_else(|| PropertyValue::string("todo"));

        let weight = Self::status_weight(&status_value);

        let decoration = Decoration {
            kind: DecorationKind::TaskCheckbox,
            target: PropertyKey::new("status").unwrap(),
            value: status_value,
            weight,
        };

        let view_properties = vec![(
            PropertyKey::new("projection").unwrap(),
            PropertyValue::string("task"),
        )];

        ProjectionViewDelta {
            decorations: vec![decoration],
            view_properties,
            conflicts: vec![],
        }
    }
}

/// The V1 task contract.
#[must_use]
pub fn task_contract() -> ProjectionContract {
    let statuses = vec![
        PropertyValue::string("todo"),
        PropertyValue::string("in-progress"),
        PropertyValue::string("done"),
        PropertyValue::string("cancelled"),
        PropertyValue::string("waiting"),
    ];
    ProjectionContract::new(ProjectionContractId::new("task").unwrap())
        .with_priority(100)
        .with_predicates(vec![
            quilt_domain::projection::PropertyPredicate::Equals {
                key: PropertyKey::new("type").unwrap(),
                value: PropertyValue::string("task"),
            },
            quilt_domain::projection::PropertyPredicate::IsSet {
                key: PropertyKey::new("status").unwrap(),
            },
            // Refine: status must be a known task status value
            quilt_domain::projection::PropertyPredicate::IsOneOf {
                key: PropertyKey::new("status").unwrap(),
                values: statuses,
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
            content: "Test task".into(),
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
    fn task_projection_contract_id() {
        let p = TaskProjection::new();
        assert_eq!(p.contract_id().as_str(), "task");
    }

    #[test]
    fn task_apply_returns_checkbox_decoration() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("done"));
            p
        });
        let ctx = ProjectionContext::page(Utc::now());
        let delta = TaskProjection::new().apply(&block, &ctx);

        assert_eq!(delta.decorations.len(), 1);
        assert_eq!(delta.decorations[0].kind, DecorationKind::TaskCheckbox);
        assert_eq!(delta.decorations[0].weight, 100); // done → weight 100
    }

    #[test]
    fn task_apply_includes_projection_property() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("todo"));
            p
        });
        let ctx = ProjectionContext::page(Utc::now());
        let delta = TaskProjection::new().apply(&block, &ctx);

        assert_eq!(delta.view_properties.len(), 1);
        let (k, v) = &delta.view_properties[0];
        assert_eq!(k.as_str(), "projection");
        assert_eq!(v, &PropertyValue::string("task"));
    }

    #[test]
    fn task_contract_matches_task_with_status() {
        let contract = task_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("in-progress"));
            p
        });
        assert!(contract.matches_block(&block));
    }

    #[test]
    fn task_contract_rejects_task_without_status() {
        let contract = task_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            // no status
            p
        });
        assert!(!contract.matches_block(&block));
    }

    #[test]
    fn task_contract_rejects_unknown_status() {
        let contract = task_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("maybe")); // not a known status
            p
        });
        assert!(!contract.matches_block(&block));
    }

    #[test]
    fn task_contract_rejects_non_task() {
        let contract = task_contract();
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("paragraph"));
            p.insert("status".into(), PropertyValue::string("done"));
            p
        });
        assert!(!contract.matches_block(&block));
    }
}
