//! Color block sections for visual thinking and content organization
//!
//! Provides styled containers with colored backgrounds/borders for:
//! - Notes, insights, and highlights
//! - Questions and open items
//! - Important warnings and callouts
//! - Custom colored sections

use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBlockVariant {
    Note,
    Insight,
    Question,
    Warning,
    Important,
    Custom,
}

impl ColorBlockVariant {
    pub fn default_color(&self) -> &'static str {
        match self {
            ColorBlockVariant::Note => "#3b82f6",
            ColorBlockVariant::Insight => "#8b5cf6",
            ColorBlockVariant::Question => "#f59e0b",
            ColorBlockVariant::Warning => "#ef4444",
            ColorBlockVariant::Important => "#22c55e",
            ColorBlockVariant::Custom => "#6b7280",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ColorBlockVariant::Note => "Note",
            ColorBlockVariant::Insight => "Insight",
            ColorBlockVariant::Question => "Question",
            ColorBlockVariant::Warning => "Warning",
            ColorBlockVariant::Important => "Important",
            ColorBlockVariant::Custom => "Custom",
        }
    }
}

#[component]
pub fn ColorBlock(
    content: String,
    variant: ColorBlockVariant,
    title: Option<String>,
    custom_color: Option<String>,
) -> impl IntoView {
    let color = custom_color.unwrap_or_else(|| variant.default_color().to_string());
    let border_color = color.clone();
    let bg_color = format!("{}15", color);

    let header_view = title.as_ref().map(|t| {
        view! {
            <div class="color-block-header" style={format!("color: {}", color)}>
                {t.clone()}
            </div>
        }
    });

    view! {
        <div
            class="color-block"
            style={format!("border-left-color: {}; background-color: {};", border_color, bg_color)}
        >
            {header_view}
            <div class="color-block-content">
                {content}
            </div>
        </div>
    }
}

#[component]
pub fn NoteBlock(content: String, title: Option<String>) -> impl IntoView {
    view! {
        <ColorBlock
            content={content}
            variant={ColorBlockVariant::Note}
            title={title}
            custom_color={None}
        />
    }
}

#[component]
pub fn InsightBlock(content: String, title: Option<String>) -> impl IntoView {
    view! {
        <ColorBlock
            content={content}
            variant={ColorBlockVariant::Insight}
            title={title}
            custom_color={None}
        />
    }
}

#[component]
pub fn QuestionBlock(content: String, title: Option<String>) -> impl IntoView {
    view! {
        <ColorBlock
            content={content}
            variant={ColorBlockVariant::Question}
            title={title}
            custom_color={None}
        />
    }
}

#[component]
pub fn WarningBlock(content: String, title: Option<String>) -> impl IntoView {
    view! {
        <ColorBlock
            content={content}
            variant={ColorBlockVariant::Warning}
            title={title}
            custom_color={None}
        />
    }
}

#[component]
pub fn ImportantBlock(content: String, title: Option<String>) -> impl IntoView {
    view! {
        <ColorBlock
            content={content}
            variant={ColorBlockVariant::Important}
            title={title}
            custom_color={None}
        />
    }
}

#[component]
pub fn CustomColorBlock(
    content: String,
    color: String,
    title: Option<String>,
) -> impl IntoView {
    view! {
        <ColorBlock
            content={content}
            variant={ColorBlockVariant::Custom}
            title={title}
            custom_color={Some(color)}
        />
    }
}
