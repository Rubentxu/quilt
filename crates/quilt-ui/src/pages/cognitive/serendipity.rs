//! Serendipity Feed — unexpected connections discovery
//!
//! Paginated view of unexpected but meaningful connections
//! discovered between knowledge blocks.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// A serendipitous connection between two blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDto {
    pub score: f32,
    pub bridge: Option<String>,
    pub source_block_id: String,
    pub target_block_id: String,
    pub connection_type: String,
}

/// Serendipity feed page component
#[component]
pub fn SerendipityFeed() -> impl IntoView {
    // Mock data for development
    let mock_connections = [
        ConnectionDto {
            score: 0.87,
            bridge: Some("Both discuss async concurrency patterns".to_string()),
            source_block_id: "block-1".to_string(),
            target_block_id: "block-2".to_string(),
            connection_type: "temporal".to_string(),
        },
        ConnectionDto {
            score: 0.76,
            bridge: Some("Rust ownership model compared to ML training".to_string()),
            source_block_id: "block-3".to_string(),
            target_block_id: "block-4".to_string(),
            connection_type: "semantic".to_string(),
        },
        ConnectionDto {
            score: 0.65,
            bridge: Some("Error handling patterns in distributed systems".to_string()),
            source_block_id: "block-5".to_string(),
            target_block_id: "block-6".to_string(),
            connection_type: "structural".to_string(),
        },
    ];

    view! {
        <div class="serendipity-feed">
            <div class="page-header">
                <h2>"✨ Serendipity Feed"</h2>
                <p class="page-subtitle">"Unexpected connections discovered"</p>
            </div>

            <div class="filters">
                <label>
                    "Min Confidence: "
                    <input type="range" min="0" max="1" step="0.1" value="0.3" />
                    <span>"0.3"</span>
                </label>
            </div>

            <div class="connection-list">
                {mock_connections.iter().map(|conn| {
                    view! {
                        <div class="connection-card">
                            <div class="connection-score">
                                <span class="score-value">{(conn.score * 100.0).round()}</span>
                                <span class="score-label">"% confidence"</span>
                            </div>
                            <div class="connection-bridge">
                                {conn.bridge.clone().unwrap_or_else(|| "No bridge description".to_string())}
                            </div>
                            <div class="connection-meta">
                                <span class="connection-type">{conn.connection_type.clone()}</span>
                                <span class="block-refs">
                                    "{conn.source_block_id[..8].to_string()}... → {conn.target_block_id[..8].to_string()}..."
                                </span>
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>

            <div class="pagination">
                <button disabled>"← Previous"</button>
                <span class="page-indicator">"Page 1"</span>
                <button disabled>"Next →"</button>
            </div>
        </div>
    }
}
