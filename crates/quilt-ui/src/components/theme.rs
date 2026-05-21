//! Theme provider component for dark/light mode
//!
//! Provides theme context and applies theme classes to the document.

use crate::state::use_app_state;
use crate::state::Theme;
use leptos::prelude::*;

/// Apply theme class to document
fn apply_theme(theme: Theme) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                match theme {
                    Theme::Light => {
                        let _ = body.class_list().remove_1("dark");
                    }
                    Theme::Dark => {
                        let _ = body.class_list().add_1("dark");
                    }
                }
            }
        }
    }
}

/// Theme provider component - wraps children and applies theme
#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
    let theme_state = use_app_state().theme;

    // Apply theme on mount and when it changes
    Effect::watch(
        move || theme_state.current.get(),
        move |theme, _, _| {
            apply_theme(*theme);
        },
        true, // Run immediately
    );

    view! {
        {children()}
    }
}

/// Theme toggle button component
#[component]
pub fn ThemeToggle() -> impl IntoView {
    view! {
        <button
            class="theme-toggle"
            on:click={move |_| use_app_state().theme.toggle()}
            title={move || {
                if use_app_state().theme.is_dark() { "Switch to light mode" } else { "Switch to dark mode" }
            }}
            data-testid="theme-toggle"
        >
            {move || if use_app_state().theme.is_dark() { "☀️" } else { "🌙" }}
        </button>
    }
}
