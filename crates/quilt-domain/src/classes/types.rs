//! Class types - entity classes with inheritance

use crate::value_objects::Uuid;

/// Class represents an entity class in the type hierarchy.
///
/// Classes form an inheritance hierarchy (Root → Page → Task, etc.)
/// and define required and default properties for blocks assigned to them.
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    /// Unique identifier
    pub id: Uuid,
    /// Database identifier (e.g., "task", "journal")
    pub db_ident: String,
    /// Display title
    pub title: String,
    /// Parent class ID (None for Root)
    pub extends: Option<Uuid>,
    /// IDs of required properties for this class
    pub required_properties: Vec<Uuid>,
    /// Default property values (property_id -> JSON value)
    pub default_properties: Vec<(Uuid, String)>,
    /// Optional icon (emoji or icon name)
    pub icon: Option<String>,
    /// Whether this is a builtin class
    pub builtin: bool,
    /// Whether this is a user-defined class
    pub user_defined: bool,
}

impl Class {
    /// Create a new class
    pub fn new(id: Uuid, db_ident: impl Into<String>, title: impl Into<String>) -> Self {
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

    /// Set the parent class (inheritance)
    pub fn with_extends(mut self, parent_id: Uuid) -> Self {
        self.extends = Some(parent_id);
        self
    }

    /// Add a required property
    pub fn with_required_property(mut self, property_id: Uuid) -> Self {
        self.required_properties.push(property_id);
        self
    }

    /// Add a default property value
    pub fn with_default_property(
        mut self,
        property_id: Uuid,
        default_json: impl Into<String>,
    ) -> Self {
        self.default_properties
            .push((property_id, default_json.into()));
        self
    }

    /// Set the icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Mark as builtin
    pub fn builtin(mut self) -> Self {
        self.builtin = true;
        self.user_defined = false;
        self
    }

    /// Mark as user-defined
    pub fn user_defined(mut self) -> Self {
        self.builtin = false;
        self.user_defined = true;
        self
    }
}

/// builtin_classes provides access to predefined system classes.
pub mod builtin_classes {
    use super::*;

    /// Root class - all classes ultimately inherit from this
    pub fn root() -> Class {
        Class::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000001")
                .expect("Invalid hardcoded root UUID"),
            "root",
            "Root",
        )
        .builtin()
    }

    /// Tag class - for tagging functionality
    pub fn tag() -> Class {
        Class::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000002")
                .expect("Invalid hardcoded tag UUID"),
            "tag",
            "Tag",
        )
        .with_extends(
            Uuid::parse_str("00000000-0000-0000-0000-000000000001")
                .expect("Invalid hardcoded root UUID"),
        )
        .builtin()
    }

    /// Page class - represents pages
    pub fn page() -> Class {
        Class::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000003")
                .expect("Invalid hardcoded page UUID"),
            "page",
            "Page",
        )
        .with_extends(
            Uuid::parse_str("00000000-0000-0000-0000-000000000001")
                .expect("Invalid hardcoded root UUID"),
        )
        .builtin()
    }

    /// Journal class - represents journal entries (extends Page)
    pub fn journal() -> Class {
        Class::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000004")
                .expect("Invalid hardcoded journal UUID"),
            "journal",
            "Journal",
        )
        .with_extends(
            Uuid::parse_str("00000000-0000-0000-0000-000000000003")
                .expect("Invalid hardcoded page UUID"),
        )
        .builtin()
    }

    /// Task class - represents tasks (extends Page)
    pub fn task() -> Class {
        Class::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000005")
                .expect("Invalid hardcoded task UUID"),
            "task",
            "Task",
        )
        .with_extends(
            Uuid::parse_str("00000000-0000-0000-0000-000000000003")
                .expect("Invalid hardcoded page UUID"),
        )
        .builtin()
    }

    /// Query class - represents saved queries
    pub fn query() -> Class {
        Class::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000006")
                .expect("Invalid hardcoded query UUID"),
            "query",
            "Query",
        )
        .builtin()
    }

    /// Property class - represents custom properties
    pub fn property_class() -> Class {
        Class::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000007")
                .expect("Invalid hardcoded property UUID"),
            "property",
            "Property",
        )
        .builtin()
    }

    /// Get all builtin classes
    pub fn all() -> Vec<Class> {
        vec![
            root(),
            tag(),
            page(),
            journal(),
            task(),
            query(),
            property_class(),
        ]
    }

    /// Get the root class ID
    pub fn root_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("Invalid hardcoded root UUID")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_builder() {
        let id = Uuid::new_v4();
        let class = Class::new(id, "custom-class", "Custom Class")
            .with_icon("📋")
            .user_defined();

        assert_eq!(class.db_ident, "custom-class");
        assert_eq!(class.title, "Custom Class");
        assert!(class.user_defined);
        assert!(!class.builtin);
        assert_eq!(class.icon, Some("📋".to_string()));
    }

    fn task_class() -> Class {
        let root_id = builtin_classes::root_id();
        Class::new(Uuid::new_v4(), "task", "Task").with_extends(root_id)
    }

    #[test]
    fn test_class_inheritance() {
        let class = task_class();
        assert!(class.extends.is_some());
        assert_eq!(class.extends.unwrap(), builtin_classes::root_id());
    }

    #[test]
    fn test_builtin_classes() {
        let classes = builtin_classes::all();
        assert_eq!(classes.len(), 7);

        let root = builtin_classes::root();
        assert_eq!(root.db_ident, "root");
        assert!(root.builtin);
        assert!(root.extends.is_none()); // Root has no parent

        let journal = builtin_classes::journal();
        assert!(journal.extends.is_some());
    }
}
