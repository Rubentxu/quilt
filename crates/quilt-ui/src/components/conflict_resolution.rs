//! Conflict resolution UI components
//!
//! Provides components for detecting and resolving sync conflicts.

use leptos::prelude::*;

/// A conflict between local and remote versions
#[derive(Debug, Clone)]
pub struct ConflictDisplay {
    /// Entity ID with the conflict
    pub entity_id: String,
    /// Entity type (block, page, etc.)
    pub entity_type: String,
    /// Local version content
    pub local_content: String,
    /// Remote version content
    pub remote_content: String,
    /// When the conflict was detected
    pub detected_at: String,
    /// Local version timestamp
    pub local_timestamp: String,
    /// Remote version timestamp
    pub remote_timestamp: String,
}

impl ConflictDisplay {
    /// Returns a summary of what changed
    pub fn diff_summary(&self) -> String {
        format!(
            "{} changed in both local and remote versions",
            self.entity_type
        )
    }
}

/// Conflict detector component - shows alert when conflicts exist
#[component]
pub fn ConflictDetector(
    conflicts: Vec<ConflictDisplay>,
    on_resolve_local: Callback<String>,
    on_resolve_remote: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="conflict-detector">
            <div class="conflict-alert">
                <span class="conflict-icon">"[!]"</span>
                <span class="conflict-count">
                    {format!("{} conflict(s) detected", conflicts.len())}
                </span>
            </div>
            <div class="conflict-list">
                    {conflicts.iter().map(|conflict| {
                        view! {
                            <ConflictCard
                                conflict=conflict.clone()
                                on_resolve_local=on_resolve_local
                                on_resolve_remote=on_resolve_remote
                            />
                        }
                    }).collect_view()}
            </div>
        </div>
    }
}

/// Single conflict card with diff view
#[component]
pub fn ConflictCard(
    conflict: ConflictDisplay,
    on_resolve_local: Callback<String>,
    on_resolve_remote: Callback<String>,
) -> impl IntoView {
    let entity_id = conflict.entity_id.clone();
    let entity_id_for_remote = conflict.entity_id.clone();
    let short_id = if entity_id.len() > 8 {
        entity_id[..8].to_string()
    } else {
        entity_id.clone()
    };

    view! {
        <div class="conflict-card">
            <div class="conflict-header">
                <span class="conflict-type">{conflict.entity_type.clone()}</span>
                <span class="conflict-id">{format!("ID: {}", short_id)}</span>
            </div>
            <div class="conflict-diff">
                <div class="diff-local">
                    <div class="diff-label">
                        <span>"Local"</span>
                        <span class="diff-time">{conflict.local_timestamp.clone()}</span>
                    </div>
                    <pre class="diff-content">{conflict.local_content.clone()}</pre>
                </div>
                <div class="diff-remote">
                    <div class="diff-label">
                        <span>"Remote"</span>
                        <span class="diff-time">{conflict.remote_timestamp.clone()}</span>
                    </div>
                    <pre class="diff-content">{conflict.remote_content.clone()}</pre>
                </div>
            </div>
            <div class="conflict-actions">
                <button
                    class="btn-resolve-local"
                    on:click=move |_| { on_resolve_local.run(entity_id.clone()); }
                >
                    "Keep Local"
                </button>
                <button
                    class="btn-resolve-remote"
                    on:click=move |_| { on_resolve_remote.run(entity_id_for_remote.clone()); }
                >
                    "Keep Remote"
                </button>
            </div>
        </div>
    }
}

/// Inline conflict marker for displaying within text
#[component]
pub fn ConflictMarker(text: String, has_conflict: bool) -> impl IntoView {
    view! {
        <span class="conflict-marker">
            {text}
            <span class="conflict-badge">
                {if has_conflict { "[C]" } else { "" }}
            </span>
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_display_diff_summary() {
        let display = ConflictDisplay {
            entity_id: "test-id".to_string(),
            entity_type: "block".to_string(),
            local_content: "local text".to_string(),
            remote_content: "remote text".to_string(),
            detected_at: "2024-01-01T00:00:00Z".to_string(),
            local_timestamp: "2024-01-01T00:00:00Z".to_string(),
            remote_timestamp: "2024-01-01T00:01:00Z".to_string(),
        };

        assert!(display.diff_summary().contains("block"));
    }
}
