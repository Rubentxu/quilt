//! Transclusion component for rendering embedded blocks
//!
//! When a block contains `((BlockId))`, this component:
//! - Shows a compact chip with preview text when collapsed
//! - Expands inline to show full block content when clicked
//! - Has a visual indicator (gray background, dotted left border)
//! - Is read-only (editing happens in the original block)

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

use crate::bridge::{get_block_content, BlockDto};

/// Transclusion state
#[derive(Debug, Clone, PartialEq)]
pub enum TransclusionState {
    Collapsed,
    Expanded,
}

/// Transclusion DTO for passing block reference info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransclusionRef {
    pub block_id: String,
    pub page_name: Option<String>,
}

/// Transclusion component - renders an embedded block reference
#[component]
pub fn Transclusion(
    block_id: String,
    page_name: Option<String>,
    #[prop(default = None)] on_navigate: Option<Callback<String, ()>>,
) -> impl IntoView {
    let state = RwSignal::new(TransclusionState::Collapsed);
    let content = RwSignal::new(Option::<BlockDto>::None);
    let is_loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);

    // Fetch block content when component mounts
    let block_id_clone = block_id.clone();
    let content_clone = content.clone();
    let is_loading_clone = is_loading.clone();
    let error_clone = error.clone();

    spawn_local(async move {
        is_loading_clone.set(true);
        error_clone.set(None);

        match get_block_content(&block_id_clone).await {
            Ok(block) => {
                content_clone.set(Some(block));
            }
            Err(e) => {
                error_clone.set(Some(e.to_string()));
            }
        }
        is_loading_clone.set(false);
    });

    let toggle_state = move |_| {
        state.update(|s| {
            *s = match s {
                TransclusionState::Collapsed => TransclusionState::Expanded,
                TransclusionState::Expanded => TransclusionState::Collapsed,
            }
        });
    };

    let navigate_to_source = move |_| {
        if let Some(callback) = &on_navigate {
            callback.run(block_id.clone());
        }
    };

    view! {
        <div
            class="transclusion"
            data-collapsed={move || state.get() == TransclusionState::Collapsed}
        >
            <Show when={move || is_loading.get()}>
                <div class="transclusion-loading">
                    <span class="transclusion-spinner">*</span>
                    <span>"Loading..."</span>
                </div>
            </Show>

            <Show when={move || error.get().is_some()}>
                <div class="transclusion-error">
                    <span>"Error"</span>
                    <span>{error.get().unwrap_or_default()}</span>
                </div>
            </Show>

            <Show when={move || content.get().is_some()}>
                <div class="transclusion-header" on:click={toggle_state}>
                    <div class="transclusion-chevron">
                        {move || if state.get() == TransclusionState::Collapsed { "\u{25B8}" } else { "\u{25BE}" }}
                    </div>
                    <div class="transclusion-icon">"[B]"</div>
                    <div class="transclusion-title">
                        {page_name.clone().unwrap_or_else(|| "Unknown".to_string())}
                    </div>
                    <Show when={move || state.get() == TransclusionState::Collapsed}>
                        <div class="transclusion-preview">
                            {let c = content.get().unwrap(); if c.content.len() > 40 {
                                format!("{}...", &c.content[..40])
                            } else {
                                c.content.clone()
                            }}
                        </div>
                    </Show>
                </div>

                <Show when={move || state.get() == TransclusionState::Expanded}>
                    <div class="transclusion-content">
                        <div class="transclusion-block-content">
                            {content.get().unwrap().content}
                        </div>
                        <div class="transclusion-actions">
                            <button
                                class="transclusion-action"
                                on:click={navigate_to_source}
                            >
                                "-> Navigate to source"
                            </button>
                        </div>
                    </div>
                </Show>
            </Show>
        </div>
    }
}

/// Transclusion chip for inline display (compact form)
#[component]
pub fn TransclusionChip(
    block_id: String,
    preview_text: String,
    on_click: Callback<String, ()>,
) -> impl IntoView {
    view! {
        <span
            class="transclusion-chip"
            on:click={move |_| on_click.run(block_id.clone())}
            title={format!("Click to expand: {}", preview_text)}
        >
            <span class="transclusion-chip-icon">"[B]"</span>
            <span class="transclusion-chip-text">
                {if preview_text.len() > 30 {
                    format!("{}...", &preview_text[..30])
                } else {
                    preview_text.clone()
                }}
            </span>
        </span>
    }
}