//! Builtin properties - predefined system properties

use super::definition::PropertyDefinition;
use super::types::{Cardinality, ClosedValue, PropertyMutability, PropertyType, PropertyVisibility, ViewContext};
use crate::value_objects::Uuid;
use std::collections::HashMap;
use std::sync::OnceLock;

/// BUILTIN_PROPERTIES contains all predefined system properties.
///
/// These are seeded into the database on migration and provide
/// the foundation for typed property system.
pub static BUILTIN_PROPERTIES: OnceLock<HashMap<String, PropertyDefinition>> = OnceLock::new();

/// Get the builtin properties map, initializing it on first access.
fn get_builtin_properties() -> &'static HashMap<String, PropertyDefinition> {
    BUILTIN_PROPERTIES.get_or_init(|| {
        let mut map = HashMap::new();

        // status property with closed values: TODO, DOING, DONE, LATER, CANCELLED
        let status_closed = vec![
            ClosedValue::new(Uuid::new_v4(), "todo", "To Do")
                .with_icon("📋")
                .with_order(1.0),
            ClosedValue::new(Uuid::new_v4(), "doing", "Doing")
                .with_icon("🏃")
                .with_order(2.0),
            ClosedValue::new(Uuid::new_v4(), "done", "Done")
                .with_icon("✅")
                .with_order(3.0),
            ClosedValue::new(Uuid::new_v4(), "later", "Later")
                .with_icon("⏰")
                .with_order(4.0),
            ClosedValue::new(Uuid::new_v4(), "cancelled", "Cancelled")
                .with_icon("❌")
                .with_order(5.0),
        ];
        let status = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/status",
            "Status",
            PropertyType::Text,
        )
        .with_cardinality(Cardinality::One)
        .with_closed_values(status_closed)
        .with_visibility(PropertyVisibility::Panel);
        map.insert("quilt.property/status".to_string(), status);

        // priority property with closed values: A, B, C
        let priority_closed = vec![
            ClosedValue::new(Uuid::new_v4(), "a", "A")
                .with_icon("🔴")
                .with_order(1.0),
            ClosedValue::new(Uuid::new_v4(), "b", "B")
                .with_icon("🟡")
                .with_order(2.0),
            ClosedValue::new(Uuid::new_v4(), "c", "C")
                .with_icon("🟢")
                .with_order(3.0),
        ];
        let priority = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/priority",
            "Priority",
            PropertyType::Text,
        )
        .with_cardinality(Cardinality::One)
        .with_closed_values(priority_closed)
        .with_visibility(PropertyVisibility::Panel);
        map.insert("quilt.property/priority".to_string(), priority);

        // deadline property (Date type)
        let deadline = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/deadline",
            "Deadline",
            PropertyType::Date,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Panel);
        map.insert("quilt.property/deadline".to_string(), deadline);

        // scheduled property (Date type)
        let scheduled = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/scheduled",
            "Scheduled",
            PropertyType::Date,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Panel);
        map.insert("quilt.property/scheduled".to_string(), scheduled);

        // url property (Url type)
        let url = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/url",
            "URL",
            PropertyType::Url,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Panel);
        map.insert("quilt.property/url".to_string(), url);

        // template property — references a template page (prefijo `template/`)
        // que define la estructura y el card-shape de un bloque. Véase ADR-0007.
        //
        // Nota: el db_ident es `template` (sin namespace prefix). Esto
        // refleja que Quilt introduce este concepto propio — no hereda
        // el namespace `quilt.property/*` que usan las propiedades
        // pre-existentes de status/priority/deadline/scheduled/url.
        // Una migración futura renombrará esas a `quilt.property/*`
        // (ver deuda técnica registrada en docs/grill/implementation-plan.md).
        //
        // El valor es un string libre (nombre de la template page) sin
        // closed values — cualquier template page es válida. Validación
        // runtime (warn si la template page no existe) ocurre en el
        // frontend via CardRenderer.
        let template =
            PropertyDefinition::new(Uuid::new_v4(), "template", "Template", PropertyType::Text)
                .with_cardinality(Cardinality::One)
                .with_visibility(PropertyVisibility::Inline);
        map.insert("template".to_string(), template);

        // ── System properties (ADR-0025: Immutable) ───────────────────────────
        // id, created_at, updated_at are system identifiers managed by the
        // domain layer (not user-editable). They are registered as
        // Immutable with PropertyVisibility::System (not searchable, not in UI).
        // Writes via PageRepository::update_properties are rejected with
        // DomainError::PropertyReadOnly(<key>).
        //
        // We construct these with the new ADR-0025 fields (visibility=System,
        // mutability=Immutable) while also setting view_context=Never to
        // preserve the legacy field value that existing tests assert on.
        let id = PropertyDefinition {
            view_context: ViewContext::Never,
            visibility: PropertyVisibility::System,
            mutability: PropertyMutability::Immutable,
            ..PropertyDefinition::new(Uuid::new_v4(), "id", "ID", PropertyType::Text)
                .with_cardinality(Cardinality::One)
        };
        map.insert("id".to_string(), id);

        let created_at = PropertyDefinition {
            view_context: ViewContext::Never,
            visibility: PropertyVisibility::System,
            mutability: PropertyMutability::Immutable,
            ..PropertyDefinition::new(Uuid::new_v4(), "created_at", "Created At", PropertyType::Text)
                .with_cardinality(Cardinality::One)
        };
        map.insert("created_at".to_string(), created_at);

        let updated_at = PropertyDefinition {
            view_context: ViewContext::Never,
            visibility: PropertyVisibility::System,
            mutability: PropertyMutability::Immutable,
            ..PropertyDefinition::new(Uuid::new_v4(), "updated_at", "Updated At", PropertyType::Text)
                .with_cardinality(Cardinality::One)
        };
        map.insert("updated_at".to_string(), updated_at);

        // tags property — free-text tags associated with a block (Cardinality::Many)
        let tags = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/tags",
            "Tags",
            PropertyType::Text,
        )
        .with_cardinality(Cardinality::Many)
        .with_visibility(PropertyVisibility::Panel);
        map.insert("quilt.property/tags".to_string(), tags);

        // created_by property — agent or user who created this block (Cardinality::One)
        let created_by = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/created_by",
            "Created By",
            PropertyType::Text,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Panel);
        map.insert("quilt.property/created_by".to_string(), created_by);

        map
    })
}

/// Get a builtin property definition by its database identifier
pub fn get_builtin_property(db_ident: &str) -> Option<&'static PropertyDefinition> {
    get_builtin_properties().get(db_ident)
}

/// Get all builtin property definitions
pub fn get_all_builtin_properties() -> Vec<&'static PropertyDefinition> {
    get_builtin_properties().values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_properties_exist() {
        let props = get_builtin_properties();
        assert!(props.contains_key("quilt.property/status"));
        assert!(props.contains_key("quilt.property/priority"));
        assert!(props.contains_key("quilt.property/deadline"));
        assert!(props.contains_key("quilt.property/scheduled"));
        assert!(props.contains_key("quilt.property/url"));
        assert!(props.contains_key("template"));
        assert!(props.contains_key("quilt.property/tags"));
        assert!(props.contains_key("quilt.property/created_by"));
    }

    #[test]
    fn test_status_has_closed_values() {
        let props = get_builtin_properties();
        let status = props.get("quilt.property/status").unwrap();
        assert!(status.has_closed_values());
        assert_eq!(status.closed_values.len(), 5);
        assert!(status.is_value_allowed("To Do"));
        assert!(status.is_value_allowed("todo"));
        assert!(status.is_value_allowed("Done"));
        assert!(!status.is_value_allowed("Invalid"));
    }

    #[test]
    fn test_priority_has_closed_values() {
        let props = get_builtin_properties();
        let priority = props.get("quilt.property/priority").unwrap();
        assert!(priority.has_closed_values());
        assert_eq!(priority.closed_values.len(), 3);
        assert!(priority.is_value_allowed("A"));
        assert!(priority.is_value_allowed("B"));
        assert!(priority.is_value_allowed("C"));
        assert!(!priority.is_value_allowed("D"));
    }

    #[test]
    fn test_date_properties_no_closed_values() {
        let props = get_builtin_properties();
        let deadline = props.get("quilt.property/deadline").unwrap();
        assert!(!deadline.has_closed_values());
        assert_eq!(deadline.property_type, PropertyType::Date);

        let scheduled = props.get("quilt.property/scheduled").unwrap();
        assert!(!scheduled.has_closed_values());
        assert_eq!(scheduled.property_type, PropertyType::Date);
    }

    #[test]
    fn test_url_property() {
        let props = get_builtin_properties();
        let url = props.get("quilt.property/url").unwrap();
        assert_eq!(url.property_type, PropertyType::Url);
        assert!(!url.has_closed_values());
    }

    #[test]
    fn test_template_property() {
        // ADR-0007: la propiedad `template` se introduce como builtin
        // de bloque. Tipo texto (nombre de template page), sin closed
        // values (cualquier template es válido), view context Block.
        // El db_ident es `template` (sin namespace prefix) porque es
        // un concepto propio de Quilt.
        let props = get_builtin_properties();
        let template = props.get("template").unwrap();
        assert_eq!(template.db_ident, "template");
        assert_eq!(template.property_type, PropertyType::Text);
        assert!(!template.has_closed_values());
    }

    // ── ADR-0025: system property builtins (id, created_at, updated_at) ──

    #[test]
    fn test_system_properties_are_registered() {
        // The three system identifiers must exist in BUILTIN_PROPERTIES.
        // They are the "system" side of the ADR-0025 mutability protection.
        let props = get_builtin_properties();
        assert!(props.contains_key("id"), "missing builtin 'id'");
        assert!(
            props.contains_key("created_at"),
            "missing builtin 'created_at'"
        );
        assert!(
            props.contains_key("updated_at"),
            "missing builtin 'updated_at'"
        );
    }

    #[test]
    fn test_system_properties_are_immutable() {
        // ADR-0025: system properties must be Immutable.
        for key in &["id", "created_at", "updated_at"] {
            let def = get_builtin_property(key).expect("system property exists");
            assert_eq!(
                def.mutability,
                PropertyMutability::Immutable,
                "{} must be Immutable",
                key
            );
        }
    }

    #[test]
    fn test_system_properties_have_system_visibility() {
        // ADR-0025: system properties have System visibility (not searchable, not in UI).
        for key in &["id", "created_at", "updated_at"] {
            let def = get_builtin_property(key).expect("system property exists");
            assert_eq!(
                def.visibility,
                PropertyVisibility::System,
                "{} must have visibility = System",
                key
            );
        }
    }

    #[test]
    fn test_system_properties_have_never_view_context() {
        // System properties are not user-editable UI properties — they
        // are not displayed in the page properties panel.
        for key in &["id", "created_at", "updated_at"] {
            let def = get_builtin_property(key).expect("system property exists");
            assert_eq!(
                def.view_context,
                ViewContext::Never,
                "{} must have view_context = Never",
                key
            );
        }
    }

    #[test]
    fn test_system_properties_are_text_type() {
        // V1 recommendation: Text for system properties (string
        // representation of UUID or RFC3339 timestamp).
        for key in &["id", "created_at", "updated_at"] {
            let def = get_builtin_property(key).expect("system property exists");
            assert_eq!(
                def.property_type,
                PropertyType::Text,
                "{} must have property_type = Text",
                key
            );
        }
    }

    #[test]
    fn test_system_properties_are_not_queryable() {
        // ADR-0025: System visibility means NOT queryable.
        for key in &["id", "created_at", "updated_at"] {
            let def = get_builtin_property(key).expect("system property exists");
            assert!(
                !def.is_queryable(),
                "{} must NOT be queryable (System visibility)",
                key
            );
        }
    }
}
