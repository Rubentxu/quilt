//! Properties editor component for block metadata
//!
//! Allows viewing and editing custom properties/key-value pairs on blocks.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct BlockProperties {
    pub properties: Vec<Property>,
}


#[component]
pub fn PropertiesEditor(
    properties: Vec<Property>,
    on_update: Callback<Vec<Property>, ()>,
) -> impl IntoView {
    let edited_properties = RwSignal::new(properties.clone());
    let is_editing = RwSignal::new(false);
    let on_update_clone = on_update;
    let original_properties = StoredValue::new(properties.clone());

    let properties_vec = move || {
        edited_properties
            .get()
            .iter()
            .enumerate()
            .map(|(i, prop)| (i, prop.clone()))
            .collect::<Vec<_>>()
    };

    view! {
        <div class="properties-editor">
            <div class="properties-header">
                <h4>"Properties"</h4>
                <Show when={move || !is_editing.get()}>
                    <button
                        class="properties-edit-btn"
                        on:click={move |_| is_editing.set(true)}
                    >
                        "Edit"
                    </button>
                </Show>
            </div>

            <Show when={move || edited_properties.get().is_empty()}>
                <p class="properties-empty">"No properties"</p>
            </Show>

            <div class="properties-list">
                <For each={properties_vec} key=|(i, _)| *i let:item>
                    {let (index, prop) = item.clone(); view! {
                        <div class="property-row">
                            <Show
                                when={move || is_editing.get()}
                                fallback={view! {
                                    <span class="property-key">{prop.key.clone()}</span>
                                    <span class="property-value">{prop.value.clone()}</span>
                                }}
                            >
                                <input
                                    type="text"
                                    class="property-key-input"
                                    value={prop.key.clone()}
                                    placeholder="Key"
                                    on:input={move |ev| {
                                        let key = event_target_value(&ev);
                                        edited_properties.update(|props| {
                                            if index < props.len() {
                                                props[index].key = key;
                                            }
                                        });
                                    }}
                                />
                                <input
                                    type="text"
                                    class="property-value-input"
                                    value={prop.value.clone()}
                                    placeholder="Value"
                                    on:input={move |ev| {
                                        let value = event_target_value(&ev);
                                        edited_properties.update(|props| {
                                            if index < props.len() {
                                                props[index].value = value;
                                            }
                                        });
                                    }}
                                />
                                <button
                                    class="property-remove-btn"
                                    on:click={move |_| {
                                        edited_properties.update(|props| {
                                            props.remove(index);
                                        });
                                    }}
                                >
                                    "×"
                                </button>
                            </Show>
                        </div>
                    }}
                </For>
            </div>

            <Show when={move || is_editing.get()}>
                <div class="properties-actions">
                    <button class="properties-add-btn" on:click={move |_| {
                        edited_properties.update(|props| {
                            props.push(Property {
                                key: String::new(),
                                value: String::new(),
                            });
                        });
                    }}>
                        "+ Add Property"
                    </button>
                    <button class="properties-save-btn" on:click={move |_| {
                        on_update_clone.run(edited_properties.get());
                        is_editing.set(false);
                    }}>
                        "Save"
                    </button>
                    <button class="properties-cancel-btn" on:click={move |_| {
                        edited_properties.set(original_properties.get_value());
                        is_editing.set(false);
                    }}>
                        "Cancel"
                    </button>
                </div>
            </Show>
        </div>
    }
}
