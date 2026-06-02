//! Class schema types — entity classes with inheritance
//!
//! Pure types extracted from `quilt-domain::classes`:
//! - Class struct with builder methods
//! - Builtin class definitions (Root, Tag, Page, Journal, Task, Query, Property)

use serde::{Deserialize, Serialize};

// ── Class ───────────────────────────────────────────────────────────

/// An entity class in the type hierarchy.
///
/// Classes form an inheritance hierarchy (Root → Page → Task, etc.)
/// and define required and default properties for blocks assigned to them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Class {
    pub id: uuid::Uuid,
    pub db_ident: String,
    pub title: String,
    pub extends: Option<uuid::Uuid>,
    pub required_properties: Vec<uuid::Uuid>,
    pub default_properties: Vec<(uuid::Uuid, String)>,
    pub icon: Option<String>,
    pub builtin: bool,
    pub user_defined: bool,
}

impl Class {
    pub fn new(id: uuid::Uuid, db_ident: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id,
            db_ident: db_ident.into(),
            title: title.into(),
            extends: None,
            required_properties: Vec::new(),
            default_properties: Vec::new(),
            icon: None,
            builtin: false,
            user_defined: true,
        }
    }

    pub fn with_extends(mut self, parent_id: uuid::Uuid) -> Self {
        self.extends = Some(parent_id);
        self
    }

    pub fn with_required_property(mut self, property_id: uuid::Uuid) -> Self {
        self.required_properties.push(property_id);
        self
    }

    pub fn with_default_property(
        mut self,
        property_id: uuid::Uuid,
        default_json: impl Into<String>,
    ) -> Self {
        self.default_properties
            .push((property_id, default_json.into()));
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn as_builtin(mut self) -> Self {
        self.builtin = true;
        self.user_defined = false;
        self
    }

    pub fn as_user_defined(mut self) -> Self {
        self.builtin = false;
        self.user_defined = true;
        self
    }
}

// ── Builtin Classes ─────────────────────────────────────────────────

/// Builtin class UUIDs (matching quilt-domain constants).
fn root_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").expect("hardcoded root UUID")
}

fn tag_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").expect("hardcoded tag UUID")
}

fn page_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000003").expect("hardcoded page UUID")
}

fn journal_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000004").expect("hardcoded journal UUID")
}

fn task_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000005").expect("hardcoded task UUID")
}

fn query_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000006").expect("hardcoded query UUID")
}

fn property_class_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000007").expect("hardcoded property UUID")
}

/// Get all builtin class definitions.
pub fn builtin_classes() -> Vec<Class> {
    let root = root_uuid();
    let page = page_uuid();

    vec![
        // Root — no parent
        Class::new(root, "root", "Root").as_builtin(),
        // Tag → Root
        Class::new(tag_uuid(), "tag", "Tag")
            .with_extends(root)
            .as_builtin(),
        // Page → Root
        Class::new(page, "page", "Page")
            .with_extends(root)
            .as_builtin(),
        // Journal → Page
        Class::new(journal_uuid(), "journal", "Journal")
            .with_extends(page)
            .as_builtin(),
        // Task → Page
        Class::new(task_uuid(), "task", "Task")
            .with_extends(page)
            .as_builtin(),
        // Query — standalone
        Class::new(query_uuid(), "query", "Query").as_builtin(),
        // Property — standalone
        Class::new(property_class_uuid(), "property", "Property").as_builtin(),
    ]
}

/// Get a builtin class by db_ident.
pub fn get_builtin_class(db_ident: &str) -> Option<Class> {
    builtin_classes()
        .into_iter()
        .find(|c| c.db_ident == db_ident)
}

/// The root class UUID.
pub fn root_class_id() -> uuid::Uuid {
    root_uuid()
}

// ── Class Validation ────────────────────────────────────────────────

/// Validate that a set of properties satisfies a class's required properties.
///
/// `properties` is a map of property-key → value. The keys may be
/// property UUIDs (as strings) or db_idents — we check each required
/// property UUID as a string key.
pub fn validate_class_required_properties(
    class: &Class,
    properties: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();

    for req_id in &class.required_properties {
        let key = req_id.to_string();
        if !properties.contains_key(&key) {
            errors.push(format!("Missing required property: {}", key));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check if adding an inheritance link would create a cycle (sync version).
///
/// Returns an error message if `child_id == parent_id` (self-reference).
/// Note: full cycle detection requires ancestors which needs a repository.
/// This function only catches self-references synchronously.
pub fn check_inheritance_self_reference(
    child_id: uuid::Uuid,
    parent_id: uuid::Uuid,
) -> Result<(), String> {
    if child_id == parent_id {
        Err("A class cannot extend itself".to_string())
    } else {
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_builder() {
        let id = uuid::Uuid::new_v4();
        let class = Class::new(id, "custom-class", "Custom Class")
            .with_icon("📋")
            .as_user_defined();

        assert_eq!(class.db_ident, "custom-class");
        assert_eq!(class.title, "Custom Class");
        assert!(class.user_defined);
        assert!(!class.builtin);
        assert_eq!(class.icon, Some("📋".to_string()));
    }

    #[test]
    fn test_class_inheritance() {
        let child =
            Class::new(uuid::Uuid::new_v4(), "child", "Child").with_extends(root_class_id());
        assert!(child.extends.is_some());
        assert_eq!(child.extends.unwrap(), root_class_id());
    }

    #[test]
    fn test_builtin_classes_count() {
        let classes = builtin_classes();
        assert_eq!(classes.len(), 7);
    }

    #[test]
    fn test_builtin_class_properties() {
        let root = get_builtin_class("root").unwrap();
        assert_eq!(root.db_ident, "root");
        assert!(root.builtin);
        assert!(root.extends.is_none());

        let journal = get_builtin_class("journal").unwrap();
        assert!(journal.extends.is_some());
        assert_eq!(journal.extends.unwrap(), page_uuid());
    }

    #[test]
    fn test_get_builtin_class_not_found() {
        assert!(get_builtin_class("nonexistent").is_none());
    }

    #[test]
    fn test_class_serialize_roundtrip() {
        let class = get_builtin_class("task").unwrap();
        let json = serde_json::to_string(&class).unwrap();
        let restored: Class = serde_json::from_str(&json).unwrap();
        assert_eq!(class, restored);
    }

    #[test]
    fn test_validate_required_properties_missing() {
        let mut class = Class::new(uuid::Uuid::new_v4(), "test", "Test");
        let prop_id = uuid::Uuid::new_v4();
        class.required_properties.push(prop_id);

        let props = serde_json::Map::new();
        let result = validate_class_required_properties(&class, &props);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains(&prop_id.to_string()));
    }

    #[test]
    fn test_validate_required_properties_present() {
        let mut class = Class::new(uuid::Uuid::new_v4(), "test", "Test");
        let prop_id = uuid::Uuid::new_v4();
        class.required_properties.push(prop_id);

        let mut props = serde_json::Map::new();
        props.insert(
            prop_id.to_string(),
            serde_json::Value::String("value".into()),
        );
        assert!(validate_class_required_properties(&class, &props).is_ok());
    }

    #[test]
    fn test_inheritance_self_reference() {
        let id = uuid::Uuid::new_v4();
        assert!(check_inheritance_self_reference(id, id).is_err());
        assert!(check_inheritance_self_reference(id, uuid::Uuid::new_v4()).is_ok());
    }
}
