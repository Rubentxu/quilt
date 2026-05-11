//! Agent panel component for sidebar chat interface
//!
//! Provides a collapsible chat interface for interacting with agents.
//! This is a placeholder component - full implementation coming soon.

use leptos::prelude::*;

/// Agent panel component for sidebar
#[component]
pub fn AgentPanel(page_name: String) -> impl IntoView {
    view! {
        <div class="agent-panel">
            {/* Header */}
            <div class="agent-panel-header">
                <span class="agent-icon">"🤖"</span>
                <span class="agent-title">"Agent"</span>
                <span class="expand-icon">"▶"</span>
            </div>

            {/* Placeholder content */}
            <div class="agent-panel-content">
                <div class="chat-empty">
                    {format!("Agent on {} - connect backend to enable", page_name)}
                </div>
            </div>
        </div>
    }
}
