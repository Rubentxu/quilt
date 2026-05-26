//! Block item component

use leptos::prelude::*;
use crate::bridge::BlockDto;

/// Simple block display component
#[component]
pub fn BlockItem(block: BlockDto) -> impl IntoView {
    view! {
        <div class="block-item">
            <span class="block-bullet">"•"</span>
            <span class="block-content">{block.content}</span>
        </div>
    }
}
