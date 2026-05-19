//! Mermaid diagram block component
//!
//! Renders Mermaid diagram source code to SVG using Mermaid.js.
//! The markdown renderer converts ```mermaid blocks to <div class="mermaid"> elements.
//! This component provides styling and fallback for when Mermaid.js isn't loaded.

use leptos::prelude::*;

#[component]
pub fn MermaidBlock(code: String) -> impl IntoView {
    view! {
        <div class="mermaid-block">
            <pre class="mermaid-source">
                <code>{code}</code>
            </pre>
        </div>
    }
}

#[component]
pub fn MermaidSource(code: String) -> impl IntoView {
    view! {
        <pre class="mermaid-source">
            <code>{code}</code>
        </pre>
    }
}
