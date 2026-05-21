//! Toast notification system for user feedback
//!
//! Provides a global toast notification system that can be accessed
//! from anywhere in the application.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Toast notification types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastType {
    pub fn icon(&self) -> &'static str {
        match self {
            ToastType::Success => "✓",
            ToastType::Error => "✕",
            ToastType::Warning => "⚠",
            ToastType::Info => "ℹ",
        }
    }

    pub fn default_timeout(&self) -> u32 {
        match self {
            ToastType::Success => 3000,
            ToastType::Error => 5000,
            ToastType::Warning => 4000,
            ToastType::Info => 3000,
        }
    }
}

/// A toast notification
#[derive(Debug, Clone)]
pub struct Toast {
    pub id: String,
    pub message: String,
    pub toast_type: ToastType,
    pub duration_ms: u32,
}

impl Toast {
    pub fn new(message: impl Into<String>, toast_type: ToastType) -> Self {
        Self {
            id: uuid_simple(),
            message: message.into(),
            toast_type,
            duration_ms: toast_type.default_timeout(),
        }
    }

    pub fn with_duration(mut self, duration_ms: u32) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Success)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Error)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Warning)
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Info)
    }
}

/// Simple UUID generator for toasts
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("toast_{:x}", now)
}

/// Toast state for the application
#[derive(Clone)]
pub struct ToastState {
    pub toasts: RwSignal<Vec<Toast>>,
}

impl Default for ToastState {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastState {
    pub fn new() -> Self {
        Self {
            toasts: RwSignal::new(vec![]),
        }
    }

    /// Add a toast notification
    pub fn add(&self, toast: Toast) {
        self.toasts.update(|t| {
            t.push(toast.clone());
            // Keep max 5 toasts
            if t.len() > 5 {
                t.remove(0);
            }
        });
    }

    /// Remove a toast by id
    pub fn remove(&self, id: &str) {
        self.toasts.update(|t| {
            t.retain(|toast| toast.id != id);
        });
    }

    /// Clear all toasts
    pub fn clear(&self) {
        self.toasts.set(vec![]);
    }

    /// Get current toasts
    pub fn get_toasts(&self) -> Vec<Toast> {
        self.toasts.get()
    }

    /// Show an error toast with the given message
    pub fn show_error(&self, message: String) {
        self.add(Toast::new(message, ToastType::Error));
    }

    /// Show a success toast with the given message
    pub fn show_success(&self, message: String) {
        self.add(Toast::new(message, ToastType::Success));
    }

    /// Show a warning toast with the given message
    pub fn show_warning(&self, message: String) {
        self.add(Toast::new(message, ToastType::Warning));
    }

    /// Show an info toast with the given message
    pub fn show_info(&self, message: String) {
        self.add(Toast::new(message, ToastType::Info));
    }
}

/// Global toast state provider
pub fn provide_toast_state() -> ToastState {
    let state = ToastState::new();
    provide_context(state.clone());
    state
}

/// Get toast state from context
pub fn use_toast_state() -> ToastState {
    expect_context()
}

/// Convenience functions for showing toasts
pub fn show_toast(toast: Toast) {
    // Note: For global toast access, use provide_toast_state at app root
    // and use the toast state directly through context
    log::debug!("Toast: {:?}", toast.message);
}

pub fn show_success(message: impl Into<String>) {
    show_toast(Toast::success(message));
}

pub fn show_error(message: impl Into<String>) {
    show_toast(Toast::error(message));
}

pub fn show_warning(message: impl Into<String>) {
    show_toast(Toast::warning(message));
}

pub fn show_info(message: impl Into<String>) {
    show_toast(Toast::info(message));
}
