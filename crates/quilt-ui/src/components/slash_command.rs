//! Slash command palette for quick template insertion and actions
//!
//! Appears when user types "/" in a block and provides:
//! - Template selection (headings, lists, tasks, etc.)
//! - Quick actions (delete, duplicate, etc.)
//! - Block references

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SlashCommand {
    pub id: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub category: CommandCategory,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CommandCategory {
    Template,
    Action,
    Reference,
}

impl CommandCategory {
    pub fn label(&self) -> &str {
        match self {
            CommandCategory::Template => "Templates",
            CommandCategory::Action => "Actions",
            CommandCategory::Reference => "References",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandGroup {
    pub category: CommandCategory,
    pub commands: Vec<SlashCommand>,
}

pub fn get_default_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand {
            id: "h1".to_string(),
            label: "Heading 1".to_string(),
            description: "Large section heading".to_string(),
            icon: "H1".to_string(),
            category: CommandCategory::Template,
            template: Some("# ".to_string()),
        },
        SlashCommand {
            id: "h2".to_string(),
            label: "Heading 2".to_string(),
            description: "Medium section heading".to_string(),
            icon: "H2".to_string(),
            category: CommandCategory::Template,
            template: Some("## ".to_string()),
        },
        SlashCommand {
            id: "h3".to_string(),
            label: "Heading 3".to_string(),
            description: "Small section heading".to_string(),
            icon: "H3".to_string(),
            category: CommandCategory::Template,
            template: Some("### ".to_string()),
        },
        SlashCommand {
            id: "bullet".to_string(),
            label: "Bullet List".to_string(),
            description: "Create a bullet point".to_string(),
            icon: "•".to_string(),
            category: CommandCategory::Template,
            template: Some("- ".to_string()),
        },
        SlashCommand {
            id: "numbered".to_string(),
            label: "Numbered List".to_string(),
            description: "Create a numbered list".to_string(),
            icon: "1.".to_string(),
            category: CommandCategory::Template,
            template: Some("1. ".to_string()),
        },
        SlashCommand {
            id: "todo".to_string(),
            label: "To-do".to_string(),
            description: "Track a task with checkbox".to_string(),
            icon: "☐".to_string(),
            category: CommandCategory::Template,
            template: Some("TODO ".to_string()),
        },
        SlashCommand {
            id: "done".to_string(),
            label: "Done".to_string(),
            description: "Mark task as complete".to_string(),
            icon: "☑".to_string(),
            category: CommandCategory::Template,
            template: Some("DONE ".to_string()),
        },
        SlashCommand {
            id: "quote".to_string(),
            label: "Quote".to_string(),
            description: "Capture a quote".to_string(),
            icon: '"'.to_string(),
            category: CommandCategory::Template,
            template: Some("> ".to_string()),
        },
        SlashCommand {
            id: "code".to_string(),
            label: "Code Block".to_string(),
            description: "Capture code snippet".to_string(),
            icon: "</>".to_string(),
            category: CommandCategory::Template,
            template: Some("```\n\n```".to_string()),
        },
        SlashCommand {
            id: "divider".to_string(),
            label: "Divider".to_string(),
            description: "Visual separator".to_string(),
            icon: "—".to_string(),
            category: CommandCategory::Template,
            template: Some("\n---\n".to_string()),
        },
        SlashCommand {
            id: "page-link".to_string(),
            label: "Page Link".to_string(),
            description: "Link to another page".to_string(),
            icon: "[[".to_string(),
            category: CommandCategory::Reference,
            template: Some("[[".to_string()),
        },
        SlashCommand {
            id: "block-ref".to_string(),
            label: "Block Reference".to_string(),
            description: "Reference another block".to_string(),
            icon: "(())".to_string(),
            category: CommandCategory::Reference,
            template: Some("(( ".to_string()),
        },
    ]
}

pub fn filter_commands(query: &str, commands: &[SlashCommand]) -> Vec<CommandGroup> {
    let query_lower = query.to_lowercase();
    let filtered: Vec<_> = commands
        .iter()
        .filter(|cmd| {
            query.is_empty()
                || cmd.label.to_lowercase().contains(&query_lower)
                || cmd.description.to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect();
    let mut groups: std::collections::HashMap<CommandCategory, Vec<SlashCommand>> =
        std::collections::HashMap::new();
    for cmd in filtered {
        groups.entry(cmd.category.clone()).or_default().push(cmd);
    }
    groups
        .into_iter()
        .map(|(category, commands)| CommandGroup { category, commands })
        .collect()
}

#[component]
pub fn SlashCommandPalette(
    is_open: bool,
    query: String,
    on_select: Callback<SlashCommand, ()>,
    on_close: Callback<(), ()>,
) -> impl IntoView {
    let all_commands = get_default_commands();
    let query_sig = Signal::derive(move || query.clone());
    let is_open_sig = Signal::derive(move || is_open);
    let commands_sig = Memo::new(move |_| filter_commands(&query_sig.get(), &all_commands));

    view! {
        <Show when={move || is_open_sig.get()}>
            <div class="slash-command-overlay">
                <div class="slash-command-palette">
                    <div class="slash-command-header">
                        <input
                            type="text"
                            class="slash-command-input"
                            placeholder="Type a command or search..."
                            value={query_sig.get()}
                        />
                    </div>
                    <div class="slash-command-list">
                        <For each={move || commands_sig.get()} key=|group| group.category.clone() let:group>
                            <div class="slash-command-group">
                                <div class="slash-command-category">{group.category.label().to_string()}</div>
                                <For each={move || group.commands.clone()} key=|cmd| cmd.id.clone() let:cmd>
                                    <button
                                        class="slash-command-item"
                                    >
                                        <span class="slash-command-icon">{cmd.icon.clone()}</span>
                                        <div class="slash-command-content">
                                            <span class="slash-command-label">{cmd.label.clone()}</span>
                                            <span class="slash-command-desc">{cmd.description.clone()}</span>
                                        </div>
                                    </button>
                                </For>
                            </div>
                        </For>
                    </div>
                </div>
            </div>
        </Show>
    }
}
