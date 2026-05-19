//! Journal inbox component — activity feed with HATEOAS references
//!
//! Shows recent activity items as an inbox/feed with:
//! - Activity items with timestamps
//! - HATEOAS-style links to related resources
//! - Activity type indicators
//! - Quick actions

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub activity_type: ActivityType,
    pub title: String,
    pub description: Option<String>,
    pub timestamp: String,
    pub links: Vec<HateoasLink>,
    pub read: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    BlockCreated,
    BlockUpdated,
    PageCreated,
    PageLinked,
    QueryRun,
    AgentAction,
    SyncComplete,
}

impl ActivityType {
    pub fn icon(&self) -> &'static str {
        match self {
            ActivityType::BlockCreated => "📝",
            ActivityType::BlockUpdated => "✏️",
            ActivityType::PageCreated => "📄",
            ActivityType::PageLinked => "🔗",
            ActivityType::QueryRun => "🔍",
            ActivityType::AgentAction => "🤖",
            ActivityType::SyncComplete => "✅",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ActivityType::BlockCreated => "Block Created",
            ActivityType::BlockUpdated => "Block Updated",
            ActivityType::PageCreated => "Page Created",
            ActivityType::PageLinked => "Page Linked",
            ActivityType::QueryRun => "Query Run",
            ActivityType::AgentAction => "Agent Action",
            ActivityType::SyncComplete => "Sync Complete",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HateoasLink {
    pub rel: String,
    pub href: String,
    pub title: String,
}

impl HateoasLink {
    pub fn page(name: &str) -> Self {
        HateoasLink {
            rel: "page".to_string(),
            href: format!("/page/{}", urlencoding::encode(name)),
            title: name.to_string(),
        }
    }

    pub fn block(id: &str) -> Self {
        HateoasLink {
            rel: "block".to_string(),
            href: format!("/block/{}", id),
            title: format!("Block {}", &id[..8.min(id.len())]),
        }
    }

    pub fn graph() -> Self {
        HateoasLink {
            rel: "graph".to_string(),
            href: "/graph".to_string(),
            title: "View in Graph".to_string(),
        }
    }
}

#[component]
pub fn JournalInbox(
    items: Vec<ActivityItem>,
    on_mark_read: Callback<String>,
    on_dismiss: Callback<String>,
) -> impl IntoView {
    let items_sig = Signal::derive(move || items.clone());

    view! {
        <div class="journal-inbox">
            <div class="inbox-header">
                <h3 class="inbox-title">"📥 Activity Feed"</h3>
                <span class="inbox-count">{items_sig.get().len()} items</span>
            </div>
            <div class="inbox-list">
                <For each={move || items_sig.get()} key=|item| item.id.clone() let:item>
                    <JournalInboxItem
                        item={item}
                        on_mark_read={on_mark_read}
                        on_dismiss={on_dismiss}
                    />
                </For>
            </div>
        </div>
    }
}

#[component]
pub fn JournalInboxItem(
    item: ActivityItem,
    on_mark_read: Callback<String>,
    on_dismiss: Callback<String>,
) -> impl IntoView {
    let links_html = item.links.iter().map(|link| {
        format!(r#"<a href="{}" class="item-link" rel="{}">{}</a>"#, link.href, link.rel, link.title)
    }).collect::<Vec<_>>().join(" ");
    let item_id = item.id.clone();
    let item_id_for_mark = item_id.clone();

    view! {
        <div class={format!("inbox-item {}", if item.read { "read" } else { "unread" })}>
            <div class="item-icon">{item.activity_type.icon()}</div>
            <div class="item-content">
                <div class="item-header">
                    <span class="item-type">{item.activity_type.label()}</span>
                    <span class="item-time">{item.timestamp.clone()}</span>
                </div>
                <div class="item-title">{item.title.clone()}</div>
                {item.description.as_ref().map(|d| {
                    view! { <div class="item-description">{d.clone()}</div> }
                })}
                <div class="item-links" inner_html={links_html.clone()}></div>
            </div>
            <div class="item-actions">
                {if !item.read {
                    Some(view! {
                        <button
                            class="btn-mark-read"
                            on:click={move |_| on_mark_read.run(item_id_for_mark.clone())}
                        >
                            "✓"
                        </button>
                    })
                } else {
                    None
                }}
                <button
                    class="btn-dismiss"
                    on:click={move |_| on_dismiss.run(item_id.clone())}
                >
                    "×"
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn ActivityItemCard(item: ActivityItem) -> impl IntoView {
    view! {
        <div class={format!("activity-card {}", if item.read { "read" } else { "unread" })}>
            <div class="card-icon">{item.activity_type.icon()}</div>
            <div class="card-body">
                <div class="card-header">
                    <span class="card-type">{item.activity_type.label()}</span>
                    <span class="card-time">{item.timestamp.clone()}</span>
                </div>
                <div class="card-title">{item.title.clone()}</div>
                {item.description.as_ref().map(|d| {
                    view! { <div class="card-description">{d.clone()}</div> }
                })}
            </div>
        </div>
    }
}
