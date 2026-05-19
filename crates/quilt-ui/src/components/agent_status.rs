//! Agent activity status bar component
//!
//! Shows real-time agent activity including:
//! - Active agents and their current tasks
//! - Recent agent actions
//! - Agent availability status

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivity {
    pub agent_id: String,
    pub agent_name: String,
    pub status: AgentStatus,
    pub current_task: Option<String>,
    pub last_activity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Thinking,
    Error,
}

impl AgentStatus {
    pub fn label(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "Idle",
            AgentStatus::Working => "Working",
            AgentStatus::Thinking => "Thinking",
            AgentStatus::Error => "Error",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "💤",
            AgentStatus::Working => "⚙️",
            AgentStatus::Thinking => "🤔",
            AgentStatus::Error => "⚠️",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "agent-idle",
            AgentStatus::Working => "agent-working",
            AgentStatus::Thinking => "agent-thinking",
            AgentStatus::Error => "agent-error",
        }
    }
}

#[component]
pub fn AgentStatusBar(activities: Vec<AgentActivity>) -> impl IntoView {
    let activities_sig = Signal::derive(move || activities.clone());

    view! {
        <div class="agent-status-bar">
            <div class="status-bar-header">
                <span class="status-bar-title">"🤖 Agents"</span>
                <span class="status-bar-count">{activities_sig.get().len()}</span>
            </div>
            <div class="status-bar-items">
                <For each={move || activities_sig.get()} key=|a| a.agent_id.clone() let:agent>
                    <div class={format!("agent-item {}", agent.status.color_class())}>
                        <span class="agent-icon">{agent.status.icon()}</span>
                        <span class="agent-name">{agent.agent_name.clone()}</span>
                        {agent.current_task.as_ref().map(|task| {
                            view! { <span class="agent-task">{task.clone()}</span> }
                        })}
                        <span class="agent-activity">{agent.last_activity.clone()}</span>
                    </div>
                </For>
            </div>
        </div>
    }
}

#[component]
pub fn AgentBadge(
    agent_name: String,
    status: AgentStatus,
) -> impl IntoView {
    view! {
        <span class={format!("agent-badge {}", status.color_class())}>
            <span class="agent-badge-icon">{status.icon()}</span>
            <span class="agent-badge-name">{agent_name}</span>
        </span>
    }
}

#[component]
pub fn AgentActivityFeed(activities: Vec<AgentActivity>) -> impl IntoView {
    let activities_sig = Signal::derive(move || activities.clone());

    view! {
        <div class="agent-activity-feed">
            <h4 class="feed-title">"Recent Agent Activity"</h4>
            <ul class="feed-list">
                <For each={move || activities_sig.get()} key=|a| a.agent_id.clone() let:activity>
                    <li class="feed-item">
                        <span class={format!("feed-icon {}", activity.status.color_class())}>
                            {activity.status.icon()}
                        </span>
                        <span class="feed-agent">{activity.agent_name.clone()}</span>
                        {activity.current_task.as_ref().map(|task| {
                            view! { <span class="feed-task">": " {task.clone()}</span> }
                        })}
                        <span class="feed-time">" - " {activity.last_activity.clone()}</span>
                    </li>
                </For>
            </ul>
        </div>
    }
}
