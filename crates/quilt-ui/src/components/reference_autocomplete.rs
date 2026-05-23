//! Reference autocomplete component for `[[Page]]` and `((Block))` references
//!
//! When user types `[[`, shows dropdown with matching page names.
//! When user types `((`, shows dropdown with matching block content.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Autocomplete item for page references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageAutocompleteItem {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
}

/// Autocomplete item for block references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAutocompleteItem {
    pub id: String,
    pub page_name: String,
    pub content_preview: String,
}

/// Reference autocomplete DTO for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReferenceAutocompleteItem {
    Page { id: String, name: String, title: Option<String> },
    Block { id: String, page_name: String, content_preview: String },
}

/// Autocomplete dropdown position
#[derive(Debug, Clone)]
pub struct AutocompletePosition {
    pub top: f64,
    pub left: f64,
}

/// Reference autocomplete state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutocompleteMode {
    None,
    PageRef,  // `[[` trigger - page name autocomplete
    BlockRef, // `((` trigger - block content autocomplete
}

/// Autocomplete dropdown component
#[component]
pub fn ReferenceAutocomplete(
    is_open: RwSignal<bool>,
    mode: RwSignal<AutocompleteMode>,
    query: RwSignal<String>,
    position: Signal<AutocompletePosition>,
    items: RwSignal<Vec<ReferenceAutocompleteItem>>,
    selected_index: RwSignal<usize>,
    on_select: Callback<ReferenceAutocompleteItem, ()>,
    on_close: Callback<(), ()>,
) -> impl IntoView {
    let items_sig = items;
    let selected_sig = selected_index;

    view! {
        <Show when={move || is_open.get()}>
            <div
                class="reference-autocomplete"
                style:top={move || format!("{}px", position.get().top)}
                style:left={move || format!("{}px", position.get().left)}
            >
                <div class="reference-autocomplete-header">
                    <span class="autocomplete-mode">
                        {move || match mode.get() {
                            AutocompleteMode::PageRef => "[[ Page ]]",
                            AutocompleteMode::BlockRef => "(( Block ))",
                            AutocompleteMode::None => "",
                        }}
                    </span>
                </div>
                <div class="reference-autocomplete-list">
                    <Show
                        when={move || !items_sig.get().is_empty()}
                        fallback={move || view! {
                            <div class="reference-autocomplete-empty">
                                "No matches found"
                            </div>
                        }}
                    >
                        <For each={move || items_sig.get().clone()} key=|item| match item.clone() {
                            ReferenceAutocompleteItem::Page { id, name, title } => id.clone(),
                            ReferenceAutocompleteItem::Block { id, page_name, .. } => id.clone(),
                        } let:item>
                            <button
                                class="reference-autocomplete-item"
                                class:selected={move || {
                                    let idx = selected_sig.get();
                                    let items = items_sig.get();
                                    items.get(idx).map(|i| match (i, &item) {
                                        (ReferenceAutocompleteItem::Page { id: i_id, .. }, ReferenceAutocompleteItem::Page { id: item_id, .. }) => i_id == item_id,
                                        (ReferenceAutocompleteItem::Block { id: i_id, .. }, ReferenceAutocompleteItem::Block { id: item_id, .. }) => i_id == item_id,
                                        _ => false,
                                    }).unwrap_or(false)
                                }}
                                on:click={move |_| {
                                    on_select.run(item.clone());
                                    is_open.set(false);
                                    query.set(String::new());
                                }}
                            >
                                {match item.clone() {
                                    ReferenceAutocompleteItem::Page { name, title, .. } => {
                                        view! {
                                            <span class="autocomplete-icon">&#x1F4C4;</span>
                                            <span class="autocomplete-text">
                                                <span class="autocomplete-name">{name.clone()}</span>
                                                <Show when={title.is_some()}>
                                                    <span class="autocomplete-title">{title.clone()}</span>
                                                </Show>
                                            </span>
                                        }
                                    },
                                    ReferenceAutocompleteItem::Block { page_name, content_preview, .. } => {
                                        view! {
                                            <span class="autocomplete-icon">&#x1F4CB;</span>
                                            <span class="autocomplete-text">
                                                <span class="autocomplete-page">{page_name}</span>
                                                <span class="autocomplete-preview">{content_preview}</span>
                                            </span>
                                        }
                                    },
                                }}
                            </button>
                        </For>
                    </Show>
                </div>
            </div>
        </Show>
    }
}

/// Detect reference trigger from text input
pub fn detect_reference_trigger(text: &str, cursor_pos: usize) -> Option<(AutocompleteMode, &str)> {
    if cursor_pos < 2 {
        return None;
    }

    // Look for `[[` before cursor
    let before_cursor = &text[..cursor_pos];
    if let Some(pos) = before_cursor.rfind("[[") {
        let query_start = pos + 2;
        let query = &before_cursor[query_start..];
        // Don't trigger if there's a closing ]] in the query
        if !query.contains("]]") {
            return Some((AutocompleteMode::PageRef, query));
        }
    }

    // Look for `((` before cursor
    if let Some(pos) = before_cursor.rfind("((") {
        let query_start = pos + 2;
        let query = &before_cursor[query_start..];
        // Don't trigger if there's a closing )) in the query
        if !query.contains("))") {
            return Some((AutocompleteMode::BlockRef, query));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_page_ref_trigger() {
        let (mode, query) = detect_reference_trigger("Hello [[World", 13).unwrap();
        assert_eq!(mode, AutocompleteMode::PageRef);
        assert_eq!(query, "World");
    }

    #[test]
    fn test_detect_page_ref_trigger_partial() {
        let (mode, query) = detect_reference_trigger("Hello [[Wor", 12).unwrap();
        assert_eq!(mode, AutocompleteMode::PageRef);
        assert_eq!(query, "Wor");
    }

    #[test]
    fn test_detect_block_ref_trigger() {
        let (mode, query) = detect_reference_trigger("See ((block-id", 15).unwrap();
        assert_eq!(mode, AutocompleteMode::BlockRef);
        assert_eq!(query, "block-id");
    }

    #[test]
    fn test_no_trigger_without_brackets() {
        assert!(detect_reference_trigger("Hello World", 11).is_none());
    }

    #[test]
    fn test_no_trigger_with_closing_brackets() {
        // Has closing ]], should not trigger
        assert!(detect_reference_trigger("Hello [[World]]", 14).is_none());
    }
}