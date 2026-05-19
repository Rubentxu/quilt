//! Query builder component for advanced block searching
//!
//! Provides both text-based DSL query input and visual query building.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryClause {
    pub field: String,
    pub operator: QueryOperator,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryOperator {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
}

impl QueryOperator {
    pub fn label(&self) -> &str {
        match self {
            QueryOperator::Equals => "=",
            QueryOperator::Contains => "contains",
            QueryOperator::StartsWith => "starts with",
            QueryOperator::EndsWith => "ends with",
            QueryOperator::GreaterThan => ">",
            QueryOperator::LessThan => "<",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub query: String,
}

#[component]
pub fn QueryBuilder(
    initial_query: String,
    on_execute: Callback<String, ()>,
) -> impl IntoView {
    let query_text = RwSignal::new(initial_query);
    let is_visual_mode = RwSignal::new(false);
    let visual_clauses = RwSignal::new(Vec::<QueryClause>::new());

    let execute_query = move |_| {
        on_execute.run(query_text.get());
    };

    let add_clause = move |_| {
        visual_clauses.update(|clauses| {
            clauses.push(QueryClause {
                field: "content".to_string(),
                operator: QueryOperator::Contains,
                value: "".to_string(),
            });
        });
    };

    let remove_clause = move |index: usize| {
        visual_clauses.update(|clauses| {
            clauses.remove(index);
        });
    };

    view! {
        <div class="query-builder">
            <div class="query-builder-header">
                <div class="query-mode-tabs">
                    <button
                        class="query-mode-tab"
                        class:active={move || !is_visual_mode.get()}
                        on:click={move |_| is_visual_mode.set(false)}
                    >
                        "Text"
                    </button>
                    <button
                        class="query-mode-tab"
                        class:active={move || is_visual_mode.get()}
                        on:click={move |_| is_visual_mode.set(true)}
                    >
                        "Visual"
                    </button>
                </div>
            </div>

            <Show when={move || !is_visual_mode.get()}>
                <div class="query-text-mode">
                    <textarea
                        class="query-input"
                        placeholder="Enter query DSL (e.g., (task todo))"
                        on:input={move |ev| query_text.set(event_target_value(&ev))}
                    >{query_text.get()}</textarea>
                    <div class="query-presets">
                        <span class="query-presets-label">"Presets:"</span>
                        <button class="query-preset-btn">"Tasks"</button>
                        <button class="query-preset-btn">"Recent"</button>
                        <button class="query-preset-btn">"Journal"</button>
                    </div>
                </div>
            </Show>

            <Show when={move || is_visual_mode.get()}>
                <div class="query-visual-mode">
                    <For each={move || visual_clauses.get().into_iter().enumerate().collect::<Vec<_>>()} key=|(_, clause)| format!("{:?}", clause.field.clone()) let:item>
                        {let (index, _clause) = item; view! {
                            <div class="query-clause">
                                <select class="query-field-select">
                                    <option value="content">"Content"</option>
                                    <option value="marker">"Marker"</option>
                                    <option value="priority">"Priority"</option>
                                    <option value="page">"Page"</option>
                                    <option value="created">"Created"</option>
                                    <option value="updated">"Updated"</option>
                                </select>
                                <select class="query-operator-select">
                                    <option value="contains">"contains"</option>
                                    <option value="=">"="</option>
                                    <option value="starts">"starts with"</option>
                                    <option value="ends">"ends with"</option>
                                    <option value=">">">"</option>
                                    <option value="<">"<"</option>
                                </select>
                                <input
                                    type="text"
                                    class="query-value-input"
                                    placeholder="Value"
                                />
                                <button
                                    class="query-remove-btn"
                                    on:click={move |_| remove_clause(index)}
                                >
                                    "x"
                                </button>
                            </div>
                        }}
                    </For>
                    <button class="query-add-btn" on:click={add_clause}>
                        "+ Add clause"
                    </button>
                </div>
            </Show>

            <div class="query-builder-footer">
                <button class="query-execute-btn" on:click={execute_query}>
                    "Execute Query"
                </button>
            </div>
        </div>
    }
}
