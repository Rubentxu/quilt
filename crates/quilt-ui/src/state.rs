//! Application state management using Leptos signals
//!
//! Provides global reactive state for:
//! - Right sidebar (open/closed, selected tab)
//! - Current block/page selection
//! - Agent activities and notifications
//! - UI preferences
//! - Theme (light/dark mode)
//! - Plugin registry

use crate::bridge::{
    self, AnnotationDto, AnnotationTypeDto, BacklinkDto, BlockInfoDto, BlockPropertiesDto,
    BridgeError, PropertyDto, RelationshipTypeDto,
};
use crate::components::agent_notification_feed::AgentNotification;
use crate::components::agent_status::AgentActivity;
use crate::components::annotations_overlay::{Annotation, AnnotationType};
use crate::components::backlinks_panel::{Backlink, RelationshipType};
use crate::components::properties_editor::Property;
use crate::components::right_sidebar::SidebarTab;
use leptos::prelude::*;
use quilt_ui_plugin::PluginRegistry;
use std::sync::Arc;

/// Theme type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}


/// Theme state
#[derive(Clone)]
pub struct ThemeState {
    pub current: RwSignal<Theme>,
}

impl ThemeState {
    pub fn new() -> Self {
        Self {
            current: RwSignal::new(Theme::Light),
        }
    }

    /// Toggle between light and dark theme
    pub fn toggle(&self) {
        self.current.update(|t| {
            *t = match *t {
                Theme::Light => Theme::Dark,
                Theme::Dark => Theme::Light,
            };
        });
    }

    /// Set theme
    pub fn set(&self, theme: Theme) {
        self.current.set(theme);
    }

    /// Check if dark mode
    pub fn is_dark(&self) -> bool {
        self.current.get() == Theme::Dark
    }
}

impl Default for ThemeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph information for multi-graph support
#[derive(Debug, Clone)]
pub struct GraphInfo {
    pub path: String,
    pub name: String,
}

impl Default for GraphInfo {
    fn default() -> Self {
        Self {
            path: String::new(),
            name: String::from("No graph"),
        }
    }
}

/// Global application state
pub struct AppState {
    /// Right sidebar state
    pub sidebar: SidebarState,
    /// Current block selection
    pub current_block: CurrentBlockState,
    /// Agent activities
    pub agents: AgentState,
    /// Notifications
    pub notifications: NotificationState,
    /// Refresh trigger - increment to trigger a refresh
    pub refresh_trigger: RwSignal<u64>,
    /// Theme state
    pub theme: ThemeState,
    /// Plugin registry for UI extensions (stored in Arc for sharing)
    pub plugins: Arc<PluginRegistry>,
    /// Current graph info (for multi-graph support)
    pub current_graph: RwSignal<Option<GraphInfo>>,
}

/// Sidebar state
#[derive(Clone)]
pub struct SidebarState {
    pub is_open: RwSignal<bool>,
    pub selected_tab: RwSignal<SidebarTab>,
    /// Mobile sidebar (hamburger menu) visibility
    pub mobile_menu_open: RwSignal<bool>,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            is_open: RwSignal::new(false),
            selected_tab: RwSignal::new(SidebarTab::Properties),
            mobile_menu_open: RwSignal::new(false),
        }
    }
}

/// Current block/page selection state
#[derive(Clone)]
pub struct CurrentBlockState {
    pub block_id: RwSignal<Option<String>>,
    pub page_id: RwSignal<Option<String>>,
    pub page_name: RwSignal<Option<String>>,
    pub properties: RwSignal<Vec<Property>>,
    pub backlinks: RwSignal<Vec<Backlink>>,
    pub annotations: RwSignal<Vec<Annotation>>,
    /// Loading states
    pub properties_loading: RwSignal<bool>,
    pub backlinks_loading: RwSignal<bool>,
    pub annotations_loading: RwSignal<bool>,
    /// Error states
    pub properties_error: RwSignal<Option<String>>,
    pub backlinks_error: RwSignal<Option<String>>,
    pub annotations_error: RwSignal<Option<String>>,
}

impl Default for CurrentBlockState {
    fn default() -> Self {
        Self {
            block_id: RwSignal::new(None),
            page_id: RwSignal::new(None),
            page_name: RwSignal::new(None),
            properties: RwSignal::new(vec![]),
            backlinks: RwSignal::new(vec![]),
            annotations: RwSignal::new(vec![]),
            properties_loading: RwSignal::new(false),
            backlinks_loading: RwSignal::new(false),
            annotations_loading: RwSignal::new(false),
            properties_error: RwSignal::new(None),
            backlinks_error: RwSignal::new(None),
            annotations_error: RwSignal::new(None),
        }
    }
}

/// Agent state for activities
#[derive(Clone)]
pub struct AgentState {
    pub activities: RwSignal<Vec<AgentActivity>>,
    pub is_connected: RwSignal<bool>,
    pub loading: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            activities: RwSignal::new(vec![]),
            is_connected: RwSignal::new(false),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
        }
    }
}

/// Notification state
#[derive(Clone)]
pub struct NotificationState {
    pub notifications: RwSignal<Vec<AgentNotification>>,
    pub unread_count: Signal<i32>,
}

impl NotificationState {
    pub fn new() -> Self {
        let notifications = RwSignal::new(vec![]);
        let unread_count = Signal::derive(move || {
            notifications
                .get()
                .iter()
                .filter(|n: &&AgentNotification| !n.read)
                .count() as i32
        });
        Self {
            notifications,
            unread_count,
        }
    }
}

impl Default for NotificationState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new global app state
    pub fn new() -> Self {
        Self {
            sidebar: SidebarState::default(),
            current_block: CurrentBlockState::default(),
            agents: AgentState::default(),
            notifications: NotificationState::new(),
            refresh_trigger: RwSignal::new(0),
            theme: ThemeState::new(),
            plugins: Arc::new(PluginRegistry::new()),
            current_graph: RwSignal::new(None),
        }
    }

    /// Open sidebar to a specific tab
    pub fn open_sidebar(&self, tab: SidebarTab) {
        self.sidebar.selected_tab.set(tab);
        self.sidebar.is_open.set(true);
    }

    /// Close sidebar
    pub fn close_sidebar(&self) {
        self.sidebar.is_open.set(false);
    }

    /// Toggle sidebar
    pub fn toggle_sidebar(&self) {
        self.sidebar.is_open.update(|v| *v = !*v);
    }

    /// Toggle mobile sidebar menu
    pub fn toggle_mobile_menu(&self) {
        self.sidebar.mobile_menu_open.update(|v| *v = !*v);
    }

    /// Close mobile sidebar menu
    pub fn close_mobile_menu(&self) {
        self.sidebar.mobile_menu_open.set(false);
    }

    /// Select a block and load its data
    /// Note: Components should call load_block_data() after this to load actual data
    pub fn select_block(&self, block_id: String, page_id: String, page_name: String) {
        self.current_block.block_id.set(Some(block_id));
        self.current_block.page_id.set(Some(page_id));
        self.current_block.page_name.set(Some(page_name));

        // Clear previous data
        self.current_block.properties.set(vec![]);
        self.current_block.backlinks.set(vec![]);
        self.current_block.annotations.set(vec![]);
        self.current_block.properties_error.set(None);
        self.current_block.backlinks_error.set(None);
        self.current_block.annotations_error.set(None);
    }

    /// Clear block selection
    pub fn clear_selection(&self) {
        self.current_block.block_id.set(None);
        self.current_block.page_id.set(None);
        self.current_block.page_name.set(None);
        self.current_block.properties.set(vec![]);
        self.current_block.backlinks.set(vec![]);
        self.current_block.annotations.set(vec![]);
        self.current_block.properties_error.set(None);
        self.current_block.backlinks_error.set(None);
        self.current_block.annotations_error.set(None);
    }

    /// Add a notification
    pub fn add_notification(&self, notification: AgentNotification) {
        self.notifications.notifications.update(|n| {
            n.insert(0, notification);
            // Keep only last 50 notifications
            if n.len() > 50 {
                n.truncate(50);
            }
        });
    }

    /// Mark notification as read
    pub fn mark_notification_read(&self, id: String) {
        self.notifications.notifications.update(|n| {
            if let Some(notif) = n.iter_mut().find(|n| n.id == id) {
                notif.read = true;
            }
        });
    }

    /// Clear all notifications
    pub fn clear_notifications(&self) {
        self.notifications.notifications.set(vec![]);
    }

    /// Update agent activities
    pub fn update_activities(&self, activities: Vec<AgentActivity>) {
        self.agents.activities.set(activities);
    }

    /// Set agent connection status
    pub fn set_agent_connected(&self, connected: bool) {
        self.agents.is_connected.set(connected);
    }

    /// Set the current graph info
    pub fn set_current_graph(&self, path: String, name: String) {
        self.current_graph.set(Some(GraphInfo { path, name }));
    }

    /// Clear the current graph (no graph open)
    pub fn clear_current_graph(&self) {
        self.current_graph.set(None);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            sidebar: self.sidebar.clone(),
            current_block: self.current_block.clone(),
            agents: self.agents.clone(),
            notifications: self.notifications.clone(),
            refresh_trigger: self.refresh_trigger,
            theme: self.theme.clone(),
            plugins: self.plugins.clone(),
            current_graph: self.current_graph,
        }
    }
}

/// Global app state - provide this at the root of the app
pub fn provide_app_state() -> AppState {
    let state = AppState::new();
    // Provide as a global resource
    provide_context(state.clone());
    state
}

/// Get the app state from context
pub fn use_app_state() -> AppState {
    expect_context()
}

// ============================================================================
// DTO Conversion Helpers
// ============================================================================

/// Convert PropertyDto to Property
impl From<PropertyDto> for Property {
    fn from(dto: PropertyDto) -> Self {
        Property {
            key: dto.key,
            value: dto.value,
        }
    }
}

/// Convert BlockPropertiesDto to Vec<Property>
impl From<BlockPropertiesDto> for Vec<Property> {
    fn from(dto: BlockPropertiesDto) -> Self {
        dto.properties.into_iter().map(Property::from).collect()
    }
}

/// Convert AnnotationDto to Annotation
impl From<AnnotationDto> for Annotation {
    fn from(dto: AnnotationDto) -> Self {
        Annotation {
            id: dto.id,
            block_id: dto.block_id,
            annotation_type: match dto.annotation_type {
                AnnotationTypeDto::Highlight => AnnotationType::Highlight,
                AnnotationTypeDto::Comment => AnnotationType::Comment,
                AnnotationTypeDto::Question => AnnotationType::Question,
                AnnotationTypeDto::Important => AnnotationType::Important,
            },
            content: dto.content,
            resolved: dto.resolved,
            created_at: dto.created_at,
        }
    }
}

/// Convert Annotation to AnnotationDto for API calls
impl From<&Annotation> for AnnotationDto {
    fn from(ann: &Annotation) -> Self {
        AnnotationDto {
            id: ann.id.clone(),
            block_id: ann.block_id.clone(),
            annotation_type: match ann.annotation_type {
                AnnotationType::Highlight => AnnotationTypeDto::Highlight,
                AnnotationType::Comment => AnnotationTypeDto::Comment,
                AnnotationType::Question => AnnotationTypeDto::Question,
                AnnotationType::Important => AnnotationTypeDto::Important,
            },
            content: ann.content.clone(),
            resolved: ann.resolved,
            created_at: ann.created_at.clone(),
        }
    }
}

/// Convert BacklinkDto to Backlink
impl From<BacklinkDto> for Backlink {
    fn from(dto: BacklinkDto) -> Self {
        Backlink {
            id: dto.id,
            source_id: dto.source_id,
            source_title: dto.source_title,
            source_preview: dto.source_preview,
            context: dto.context,
            relationship_type: match dto.relationship_type {
                RelationshipTypeDto::Direct => RelationshipType::Direct,
                RelationshipTypeDto::Transitive => RelationshipType::Transitive,
                RelationshipTypeDto::Semantic => RelationshipType::Semantic,
            },
            created_at: dto.created_at,
            provenance_score: dto.provenance_score,
        }
    }
}

/// Convert RelationshipType to RelationshipTypeDto
impl From<RelationshipType> for RelationshipTypeDto {
    fn from(rt: RelationshipType) -> Self {
        match rt {
            RelationshipType::Direct => RelationshipTypeDto::Direct,
            RelationshipType::Transitive => RelationshipTypeDto::Transitive,
            RelationshipType::Semantic => RelationshipTypeDto::Semantic,
        }
    }
}

// ============================================================================
// Data Loading Methods
// ============================================================================

impl AppState {
    /// Load all data for a block (properties, annotations)
    pub async fn load_block_data(&self, block_id: &str) {
        // Load properties
        self.current_block.properties_loading.set(true);
        self.current_block.properties_error.set(None);
        match bridge::get_block_properties(block_id).await {
            Ok(dto) => {
                self.current_block.properties.set(dto.into());
            }
            Err(e) => {
                self.current_block.properties_error.set(Some(e.to_string()));
            }
        }
        self.current_block.properties_loading.set(false);

        // Load annotations
        self.current_block.annotations_loading.set(true);
        self.current_block.annotations_error.set(None);
        match bridge::get_block_annotations(block_id).await {
            Ok(dtos) => {
                self.current_block
                    .annotations
                    .set(dtos.into_iter().map(Annotation::from).collect());
            }
            Err(e) => {
                self.current_block
                    .annotations_error
                    .set(Some(e.to_string()));
            }
        }
        self.current_block.annotations_loading.set(false);

        // Load backlinks if we have a page name
        if let Some(page_name) = self.current_block.page_name.get() {
            let _ = self.load_backlinks(&page_name).await;
        }
    }

    /// Load properties for a block from the backend
    pub async fn load_properties(&self, block_id: &str) -> Result<(), BridgeError> {
        self.current_block.properties_loading.set(true);
        self.current_block.properties_error.set(None);

        let result = match bridge::get_block_properties(block_id).await {
            Ok(dto) => {
                self.current_block.properties.set(dto.into());
                Ok(())
            }
            Err(e) => {
                self.current_block.properties_error.set(Some(e.to_string()));
                Err(e)
            }
        };

        self.current_block.properties_loading.set(false);
        result
    }

    /// Save properties for a block to the backend
    pub async fn save_properties(&self, block_id: &str) -> Result<(), BridgeError> {
        let properties: Vec<PropertyDto> = self
            .current_block
            .properties
            .get()
            .iter()
            .map(|p| PropertyDto {
                key: p.key.clone(),
                value: p.value.clone(),
            })
            .collect();

        match bridge::update_block_properties(block_id, properties).await {
            Ok(dto) => {
                self.current_block.properties.set(dto.into());
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Load annotations for a block from the backend
    pub async fn load_annotations(&self, block_id: &str) -> Result<(), BridgeError> {
        self.current_block.annotations_loading.set(true);
        self.current_block.annotations_error.set(None);

        let result = match bridge::get_block_annotations(block_id).await {
            Ok(dtos) => {
                self.current_block
                    .annotations
                    .set(dtos.into_iter().map(Annotation::from).collect());
                Ok(())
            }
            Err(e) => {
                self.current_block
                    .annotations_error
                    .set(Some(e.to_string()));
                Err(e)
            }
        };

        self.current_block.annotations_loading.set(false);
        result
    }

    /// Create a new annotation
    pub async fn create_annotation(
        &self,
        block_id: &str,
        annotation_type: AnnotationType,
        content: &str,
    ) -> Result<Annotation, BridgeError> {
        let dto_type = match annotation_type {
            AnnotationType::Highlight => AnnotationTypeDto::Highlight,
            AnnotationType::Comment => AnnotationTypeDto::Comment,
            AnnotationType::Question => AnnotationTypeDto::Question,
            AnnotationType::Important => AnnotationTypeDto::Important,
        };

        match bridge::create_annotation(block_id, dto_type, content).await {
            Ok(dto) => {
                let annotation = Annotation::from(dto);
                self.current_block.annotations.update(|a| {
                    a.insert(0, annotation.clone());
                });
                Ok(annotation)
            }
            Err(e) => Err(e),
        }
    }

    /// Resolve or unresolve an annotation
    pub async fn resolve_annotation(
        &self,
        annotation_id: &str,
        resolved: bool,
    ) -> Result<(), BridgeError> {
        match bridge::resolve_annotation(annotation_id, resolved).await {
            Ok(dto) => {
                let annotation = Annotation::from(dto);
                self.current_block.annotations.update(|anns| {
                    if let Some(a) = anns.iter_mut().find(|a| a.id == annotation_id) {
                        *a = annotation;
                    }
                });
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Delete an annotation
    pub async fn delete_annotation(&self, annotation_id: &str) -> Result<(), BridgeError> {
        match bridge::delete_annotation(annotation_id).await {
            Ok(_) => {
                self.current_block.annotations.update(|anns| {
                    anns.retain(|a| a.id != annotation_id);
                });
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Load backlinks for a page from the backend
    pub async fn load_backlinks(&self, page_name: &str) -> Result<(), BridgeError> {
        self.current_block.backlinks_loading.set(true);
        self.current_block.backlinks_error.set(None);

        let result = match bridge::get_page_backlinks(page_name).await {
            Ok(dtos) => {
                self.current_block
                    .backlinks
                    .set(dtos.into_iter().map(Backlink::from).collect());
                Ok(())
            }
            Err(e) => {
                self.current_block.backlinks_error.set(Some(e.to_string()));
                Err(e)
            }
        };

        self.current_block.backlinks_loading.set(false);
        result
    }

    /// Trigger a refresh of all current data
    pub async fn refresh(&self) {
        self.refresh_trigger.update(|v| *v += 1);

        if let Some(block_id) = self.current_block.block_id.get() {
            self.load_block_data(&block_id).await;
        }
    }

    /// Check if any data is currently loading
    pub fn is_loading(&self) -> bool {
        self.current_block.properties_loading.get()
            || self.current_block.backlinks_loading.get()
            || self.current_block.annotations_loading.get()
            || self.agents.loading.get()
    }

    // ============================================================================
    // Trash / Recycle Bin Methods
    // ============================================================================

    /// Delete a block (soft delete / move to trash)
    pub async fn delete_block(&self, block_id: &str) -> Result<(), BridgeError> {
        bridge::delete_block(block_id).await
    }

    /// Restore a block from trash
    pub async fn restore_block(&self, block_id: &str) -> Result<(), BridgeError> {
        bridge::restore_block(block_id).await
    }

    /// Get all blocks in the recycle bin / trash
    pub async fn get_trash(&self) -> Result<Vec<BlockInfoDto>, BridgeError> {
        bridge::get_recycle_bin().await
    }

    /// Permanently delete a block from trash
    pub async fn hard_delete_block(&self, block_id: &str) -> Result<(), BridgeError> {
        bridge::hard_delete_block(block_id).await
    }
}
