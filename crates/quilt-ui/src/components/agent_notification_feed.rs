//! Agent notification feed component for real-time activity stream
//!
//! Shows agent notifications and activity including:
//! - Agent actions (searching, creating, updating blocks)
//! - AI insight notifications
//! - Error alerts
//! - Timestamped activity items

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Agent notification types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
    Insight,
}

impl NotificationType {
    pub fn icon(&self) -> &'static str {
        match self {
            NotificationType::Info => "ℹ️",
            NotificationType::Success => "✅",
            NotificationType::Warning => "⚠️",
            NotificationType::Error => "❌",
            NotificationType::Insight => "💡",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            NotificationType::Info => "notification-info",
            NotificationType::Success => "notification-success",
            NotificationType::Warning => "notification-warning",
            NotificationType::Error => "notification-error",
            NotificationType::Insight => "notification-insight",
        }
    }
}

/// A single agent notification item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNotification {
    pub id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub timestamp: i64, // Unix timestamp in milliseconds
    pub read: bool,
    pub block_id: Option<String>,
    pub page_name: Option<String>,
}

impl AgentNotification {
    pub fn new_info(title: &str, message: &str) -> Self {
        Self {
            id: format!(
                "notif-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ),
            notification_type: NotificationType::Info,
            title: title.to_string(),
            message: message.to_string(),
            agent_id: None,
            agent_name: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            read: false,
            block_id: None,
            page_name: None,
        }
    }

    pub fn new_insight(agent_name: &str, title: &str, message: &str) -> Self {
        Self {
            id: format!(
                "notif-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ),
            notification_type: NotificationType::Insight,
            title: title.to_string(),
            message: message.to_string(),
            agent_id: None,
            agent_name: Some(agent_name.to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            read: false,
            block_id: None,
            page_name: None,
        }
    }

    pub fn new_error(agent_name: &str, message: &str) -> Self {
        Self {
            id: format!(
                "notif-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ),
            notification_type: NotificationType::Error,
            title: "Agent Error".to_string(),
            message: message.to_string(),
            agent_id: None,
            agent_name: Some(agent_name.to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            read: false,
            block_id: None,
            page_name: None,
        }
    }
}

/// Activity feed item (for the activity stream)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub activity_type: ActivityType,
    pub description: String,
    pub agent_name: String,
    pub timestamp: i64, // Unix timestamp in milliseconds
    pub block_id: Option<String>,
    pub page_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    Search,
    Create,
    Update,
    Delete,
    Link,
    Query,
    Analyze,
}

impl ActivityType {
    pub fn icon(&self) -> &'static str {
        match self {
            ActivityType::Search => "🔍",
            ActivityType::Create => "➕",
            ActivityType::Update => "✏️",
            ActivityType::Delete => "🗑️",
            ActivityType::Link => "🔗",
            ActivityType::Query => "💬",
            ActivityType::Analyze => "🧠",
        }
    }
}

/// Format timestamp for display
fn format_timestamp(ts_millis: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let duration = now - ts_millis;
    let seconds = duration / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else {
        format!("{}d ago", days)
    }
}

/// Agent notification feed component
#[component]
pub fn AgentNotificationFeed(
    notifications: Vec<AgentNotification>,
    on_mark_read: Option<Callback<String, ()>>,
    on_clear: Option<Callback<(), ()>>,
) -> impl IntoView {
    let notifications_clone = notifications.clone();
    let notifications_sig = Signal::derive(move || notifications_clone.clone());
    let unread_count = move || notifications_sig.get().iter().filter(|n| !n.read).count();

    view! {
        <div class="agent-notification-feed">
            <div class="notification-header">
                <h3>"Notifications"</h3>
                <Show when={move || unread_count() > 0}>
                    <span class="notification-badge">{move || unread_count()}</span>
                </Show>
            </div>

            <Show when={move || notifications_sig.get().is_empty()}>
                <div class="notification-empty">
                    <p>"No notifications"</p>
                </div>
            </Show>

            <div class="notification-list">
                <For each={move || notifications_sig.get()} key=|n| n.id.clone() let:notification>
                    <div
                        class={format!("notification-item {}", notification.notification_type.color_class())}
                        class:unread={notification.read}
                        on:click={move |_| {
                            if let Some(callback) = &on_mark_read {
                                callback.run(notification.id.clone());
                            }
                        }}
                    >
                        <span class="notification-icon">
                            {notification.notification_type.icon()}
                        </span>
                        <div class="notification-content">
                            <div class="notification-title">{notification.title.clone()}</div>
                            <div class="notification-message">{notification.message.clone()}</div>
                            <div class="notification-meta">
                                <span class="notification-agent">
                                    {notification.agent_name.clone().unwrap_or_default()}
                                </span>
                                <span class="notification-time">
                                    {format_timestamp(notification.timestamp)}
                                </span>
                            </div>
                        </div>
                    </div>
                </For>
            </div>

            <Show when={move || !notifications.is_empty() && on_clear.is_some()}>
                <div class="notification-footer">
                    <button
                        class="notification-clear-btn"
                        on:click={move |_| {
                            if let Some(callback) = &on_clear {
                                callback.run(());
                            }
                        }}
                    >
                        "Clear all"
                    </button>
                </div>
            </Show>
        </div>
    }
}

/// Agent activity stream component
#[component]
pub fn AgentActivityStream(activities: Vec<ActivityItem>) -> impl IntoView {
    let activities_sig = Signal::derive(move || activities.clone());

    view! {
        <div class="agent-activity-stream">
            <div class="activity-header">
                <h4>"Activity"</h4>
            </div>

            <Show when={move || activities_sig.get().is_empty()}>
                <div class="activity-empty">
                    <p>"No recent activity"</p>
                </div>
            </Show>

            <div class="activity-list">
                <For each={move || activities_sig.get()} key=|a| a.id.clone() let:item>
                    <div class="activity-item">
                        <span class="activity-icon">
                            {item.activity_type.icon()}
                        </span>
                        <div class="activity-content">
                            <span class="activity-agent">{item.agent_name.clone()}</span>
                            <span class="activity-description">{item.description.clone()}</span>
                        </div>
                        <span class="activity-time">{format_timestamp(item.timestamp)}</span>
                    </div>
                </For>
            </div>
        </div>
    }
}
