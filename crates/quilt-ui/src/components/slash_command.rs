//! Slash command system for inline block commands.
//!
//! Provides a registry of typed slash commands (TODO, priority, dates, formatting)
//! with filtered search and execution model.
//!
//! # Architecture
//!
//! - `SlashCommand`: immutable command definition with execute callback
//! - `SlashContext`: current block state passed to execute
//! - `SlashCommandAction`: what the command wants the editor to do
//! - `filter_commands`: fuzzy filter by query string
//! - `get_default_commands`: all built-in commands

use std::sync::Arc;

/// Category grouping for slash commands in the dropdown UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlashCommandCategory {
    TaskStatus,
    Priority,
    Date,
    Format,
    Structure,
}

impl SlashCommandCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TaskStatus => "Task Status",
            Self::Priority => "Priority",
            Self::Date => "Date",
            Self::Format => "Format",
            Self::Structure => "Structure",
        }
    }
}

/// Context passed to a slash command's execute callback.
///
/// This is a snapshot of the block state at the time the command is executed.
pub struct SlashContext {
    pub content: String,
    pub cursor_offset: usize,
    pub block_id: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
}

/// Action returned by a slash command to tell the editor what to do.
///
/// Commands return an action (or composite of actions) rather than mutating
/// state directly, so the editor component can apply the changes within its
/// own update cycle and maintain undo/redo consistency.
#[derive(Debug, Clone)]
pub enum SlashCommandAction {
    /// Replace the full block content and set cursor position.
    UpdateContent {
        content: String,
        cursor: usize,
    },
    /// Insert text at the current cursor position (preserving existing content).
    InsertAtCursor {
        text: String,
    },
    /// Set the block's marker (e.g., "todo", "doing", "done").
    SetMarker(String),
    /// Set the block's priority (e.g., "A", "B", "C").
    SetPriority(String),
    /// Activate an autocomplete trigger by inserting text into the editor
    /// (e.g., "[[", "((").
    ActivateAutocomplete(String),
    /// Perform multiple actions in sequence (applied left to right).
    Composite(Vec<SlashCommandAction>),
}

/// A single slash command definition.
pub struct SlashCommand {
    pub id: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub category: SlashCommandCategory,
    /// The execute callback receives the current block context and returns
    /// a `SlashCommandAction` describing what to do.
    pub execute: Arc<dyn Fn(&SlashContext) -> SlashCommandAction + Send + Sync>,
}

impl std::fmt::Debug for SlashCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlashCommand")
            .field("id", &self.id)
            .field("label", &self.label)
            .finish()
    }
}

/// Group of commands sharing a category, used for rendering grouped dropdowns.
pub struct CommandGroup {
    pub category: SlashCommandCategory,
    pub commands: Vec<usize>,
}

// ── Helper: insert text at cursor position ──

fn insert_at_cursor(ctx: &SlashContext, text: &str) -> SlashCommandAction {
    let cursor = ctx.cursor_offset.min(ctx.content.len());
    let new_content = format!(
        "{}{}{}",
        &ctx.content[..cursor],
        text,
        &ctx.content[cursor..]
    );
    let new_cursor = cursor + text.len();
    SlashCommandAction::UpdateContent {
        content: new_content,
        cursor: new_cursor,
    }
}

// ── Helper: strip existing heading prefix ──

fn strip_heading(content: &str) -> String {
    let trimmed = content.trim_start();
    if trimmed.starts_with("### ") {
        trimmed[4..].to_string()
    } else if trimmed.starts_with("## ") {
        trimmed[3..].to_string()
    } else if trimmed.starts_with("# ") {
        trimmed[2..].to_string()
    } else {
        content.to_string()
    }
}

// ── Helper: strip existing marker prefix (e.g., "TODO ", "DOING ", "DONE ") ──

#[allow(dead_code)]
fn strip_marker(content: &str) -> String {
    let trimmed = content.trim_start();
    // Check for common markers at the beginning
    for marker in &["TODO ", "DOING ", "DONE ", "LATER ", "NOW ", "CANCELLED "] {
        if trimmed.starts_with(marker) || trimmed.starts_with(&marker.to_lowercase()) {
            return trimmed[marker.len()..].to_string();
        }
    }
    content.to_string()
}

// ── All default commands ──

/// Returns the built-in list of slash commands.
///
/// These mirror Logseq's common slash commands and are grouped by category
/// for the dropdown UI.
pub fn get_default_commands() -> Vec<SlashCommand> {
    vec![
        // ── Task Status ──
        SlashCommand {
            id: "todo".into(),
            label: "TODO".into(),
            description: "Set block status to TODO".into(),
            icon: "○".into(),
            category: SlashCommandCategory::TaskStatus,
            execute: Arc::new(|_ctx| {
                SlashCommandAction::Composite(vec![SlashCommandAction::SetMarker("todo".into())])
            }),
        },
        SlashCommand {
            id: "doing".into(),
            label: "DOING".into(),
            description: "Set block status to DOING".into(),
            icon: "●".into(),
            category: SlashCommandCategory::TaskStatus,
            execute: Arc::new(|_ctx| {
                SlashCommandAction::Composite(vec![SlashCommandAction::SetMarker("doing".into())])
            }),
        },
        SlashCommand {
            id: "done".into(),
            label: "DONE".into(),
            description: "Set block status to DONE".into(),
            icon: "✓".into(),
            category: SlashCommandCategory::TaskStatus,
            execute: Arc::new(|_ctx| {
                SlashCommandAction::Composite(vec![SlashCommandAction::SetMarker("done".into())])
            }),
        },
        // ── Priority ──
        SlashCommand {
            id: "priority-a".into(),
            label: "Priority A".into(),
            description: "Set block priority to A (highest)".into(),
            icon: "P1".into(),
            category: SlashCommandCategory::Priority,
            execute: Arc::new(|_ctx| SlashCommandAction::SetPriority("A".into())),
        },
        SlashCommand {
            id: "priority-b".into(),
            label: "Priority B".into(),
            description: "Set block priority to B (medium)".into(),
            icon: "P2".into(),
            category: SlashCommandCategory::Priority,
            execute: Arc::new(|_ctx| SlashCommandAction::SetPriority("B".into())),
        },
        SlashCommand {
            id: "priority-c".into(),
            label: "Priority C".into(),
            description: "Set block priority to C (lowest)".into(),
            icon: "P3".into(),
            category: SlashCommandCategory::Priority,
            execute: Arc::new(|_ctx| SlashCommandAction::SetPriority("C".into())),
        },
        // ── Date ──
        SlashCommand {
            id: "today".into(),
            label: "Today".into(),
            description: "Insert today's date as page reference".into(),
            icon: "📅".into(),
            category: SlashCommandCategory::Date,
            execute: Arc::new(|ctx| {
                let today = chrono::Local::now()
                    .format("%B %d, %Y")
                    .to_string();
                let date_ref = format!("[[{}]]", today);
                insert_at_cursor(ctx, &date_ref)
            }),
        },
        SlashCommand {
            id: "tomorrow".into(),
            label: "Tomorrow".into(),
            description: "Insert tomorrow's date as page reference".into(),
            icon: "📅".into(),
            category: SlashCommandCategory::Date,
            execute: Arc::new(|ctx| {
                let tomorrow = chrono::Local::now()
                    .checked_add_signed(chrono::TimeDelta::days(1))
                    .unwrap_or_else(|| chrono::Local::now())
                    .format("%B %d, %Y")
                    .to_string();
                let date_ref = format!("[[{}]]", tomorrow);
                insert_at_cursor(ctx, &date_ref)
            }),
        },
        SlashCommand {
            id: "deadline".into(),
            label: "Deadline".into(),
            description: "Insert deadline property with today's date".into(),
            icon: "⏰".into(),
            category: SlashCommandCategory::Date,
            execute: Arc::new(|ctx| {
                let today = chrono::Local::now()
                    .format("%B %d, %Y")
                    .to_string();
                let text = format!("deadline:: [[{}]]", today);
                insert_at_cursor(ctx, &text)
            }),
        },
        SlashCommand {
            id: "scheduled".into(),
            label: "Scheduled".into(),
            description: "Insert scheduled property with today's date".into(),
            icon: "📆".into(),
            category: SlashCommandCategory::Date,
            execute: Arc::new(|ctx| {
                let today = chrono::Local::now()
                    .format("%B %d, %Y")
                    .to_string();
                let text = format!("scheduled:: [[{}]]", today);
                insert_at_cursor(ctx, &text)
            }),
        },
        // ── Structure ──
        SlashCommand {
            id: "page-ref".into(),
            label: "Page Reference".into(),
            description: "Insert [[ to link to a page".into(),
            icon: "[[".into(),
            category: SlashCommandCategory::Structure,
            execute: Arc::new(|ctx| {
                insert_at_cursor(ctx, "[[")
            }),
        },
        SlashCommand {
            id: "block-embed".into(),
            label: "Block Embed".into(),
            description: "Insert (( to embed a block".into(),
            icon: "(())".into(),
            category: SlashCommandCategory::Structure,
            execute: Arc::new(|ctx| {
                insert_at_cursor(ctx, "((")
            }),
        },
        SlashCommand {
            id: "code-block".into(),
            label: "Code Block".into(),
            description: "Insert a code block".into(),
            icon: "</>".into(),
            category: SlashCommandCategory::Structure,
            execute: Arc::new(|_ctx| {
                SlashCommandAction::UpdateContent {
                    content: "```\n\n```".into(),
                    cursor: 4, // between the backticks
                }
            }),
        },
        // ── Format ──
        SlashCommand {
            id: "h1".into(),
            label: "Heading 1".into(),
            description: "Large section heading".into(),
            icon: "H1".into(),
            category: SlashCommandCategory::Format,
            execute: Arc::new(|ctx| {
                let clean = strip_heading(&ctx.content);
                let new_content = format!("# {}", clean);
                let cursor = new_content.len();
                SlashCommandAction::UpdateContent {
                    content: new_content,
                    cursor,
                }
            }),
        },
        SlashCommand {
            id: "h2".into(),
            label: "Heading 2".into(),
            description: "Medium section heading".into(),
            icon: "H2".into(),
            category: SlashCommandCategory::Format,
            execute: Arc::new(|ctx| {
                let clean = strip_heading(&ctx.content);
                let new_content = format!("## {}", clean);
                let cursor = new_content.len();
                SlashCommandAction::UpdateContent {
                    content: new_content,
                    cursor,
                }
            }),
        },
        SlashCommand {
            id: "h3".into(),
            label: "Heading 3".into(),
            description: "Small section heading".into(),
            icon: "H3".into(),
            category: SlashCommandCategory::Format,
            execute: Arc::new(|ctx| {
                let clean = strip_heading(&ctx.content);
                let new_content = format!("### {}", clean);
                let cursor = new_content.len();
                SlashCommandAction::UpdateContent {
                    content: new_content,
                    cursor,
                }
            }),
        },
        SlashCommand {
            id: "normal-text".into(),
            label: "Normal Text".into(),
            description: "Remove heading formatting".into(),
            icon: "T".into(),
            category: SlashCommandCategory::Format,
            execute: Arc::new(|ctx| {
                let new_content = strip_heading(&ctx.content);
                SlashCommandAction::UpdateContent {
                    cursor: new_content.len(),
                    content: new_content,
                }
            }),
        },
    ]
}

/// Filter commands by a query string, grouping results by category.
///
/// Matching is case-insensitive against both `label` and `description`.
/// An empty query returns all commands (showing the full palette).
pub fn filter_commands(
    query: &str,
    commands: &[SlashCommand],
) -> Vec<CommandGroup> {
    let query_lower = query.to_lowercase();
    let mut matched_indices: Vec<usize> = commands
        .iter()
        .enumerate()
        .filter(|(_, cmd)| {
            query.is_empty()
                || cmd.label.to_lowercase().contains(&query_lower)
                || cmd.description.to_lowercase().contains(&query_lower)
        })
        .map(|(i, _)| i)
        .collect();

    // Sort by category for stable grouping
    matched_indices.sort_by_key(|&i| commands[i].category as u8);

    let mut groups: Vec<CommandGroup> = Vec::new();
    let mut current_cat: Option<SlashCommandCategory> = None;

    for idx in matched_indices {
        let cat = commands[idx].category;
        if current_cat != Some(cat) {
            current_cat = Some(cat);
            groups.push(CommandGroup {
                category: cat,
                commands: Vec::new(),
            });
        }
        if let Some(last) = groups.last_mut() {
            last.commands.push(idx);
        }
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_commands() -> Vec<SlashCommand> {
        get_default_commands()
    }

    #[test]
    fn test_default_commands_count() {
        let cmds = make_test_commands();
        assert!(!cmds.is_empty(), "Should have at least one default command");
        assert!(cmds.len() >= 14, "Should have all core commands");
    }

    #[test]
    fn test_filter_commands_empty_query_returns_all() {
        let cmds = make_test_commands();
        let groups = filter_commands("", &cmds);
        let total: usize = groups.iter().map(|g| g.commands.len()).sum();
        assert_eq!(total, cmds.len(), "Empty query should return all commands");
    }

    #[test]
    fn test_filter_commands_by_label() {
        let cmds = make_test_commands();
        let groups = filter_commands("todo", &cmds);
        let total: usize = groups.iter().map(|g| g.commands.len()).sum();
        assert!(total > 0, "'todo' should match at least TODO command");
        // First command in first group should be the TODO command
        let first_idx = groups[0].commands[0];
        assert!(cmds[first_idx].label.to_lowercase().contains("todo"));
    }

    #[test]
    fn test_filter_commands_by_description() {
        let cmds = make_test_commands();
        let groups = filter_commands("status", &cmds);
        let total: usize = groups.iter().map(|g| g.commands.len()).sum();
        assert!(total > 0, "'status' should match task status commands");
    }

    #[test]
    fn test_filter_commands_case_insensitive() {
        let cmds = make_test_commands();
        let groups_upper = filter_commands("TODO", &cmds);
        let groups_lower = filter_commands("todo", &cmds);
        let total_upper: usize = groups_upper.iter().map(|g| g.commands.len()).sum();
        let total_lower: usize = groups_lower.iter().map(|g| g.commands.len()).sum();
        assert_eq!(total_upper, total_lower, "Case should not matter");
    }

    #[test]
    fn test_filter_commands_no_match() {
        let cmds = make_test_commands();
        let groups = filter_commands("zzzznonexistent", &cmds);
        let total: usize = groups.iter().map(|g| g.commands.len()).sum();
        assert_eq!(total, 0, "No commands should match garbage query");
    }

    #[test]
    fn test_filter_commands_preserves_groups() {
        let cmds = make_test_commands();
        let groups = filter_commands("", &cmds);
        // Should have multiple categories
        assert!(
            groups.len() > 1,
            "Should have multiple category groups: {}",
            groups.len()
        );
        // Categories should be distinct
        let cat_ids: Vec<u8> = groups.iter().map(|g| g.category as u8).collect();
        let mut sorted = cat_ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(cat_ids, sorted, "Categories should be unique in order");
    }

    #[test]
    fn test_todo_command_action() {
        let cmds = make_test_commands();
        let todo_cmd = cmds.iter().find(|c| c.id == "todo").unwrap();
        let ctx = SlashContext {
            content: "hello".into(),
            cursor_offset: 0,
            block_id: "test".into(),
            marker: None,
            priority: None,
        };
        let action = (todo_cmd.execute)(&ctx);
        match action {
            SlashCommandAction::Composite(actions) => {
                assert_eq!(actions.len(), 1);
                match &actions[0] {
                    SlashCommandAction::SetMarker(m) => assert_eq!(m, "todo"),
                    _ => panic!("Expected SetMarker action"),
                }
            }
            _ => panic!("Expected Composite action"),
        }
    }

    #[test]
    fn test_today_command_action() {
        let cmds = make_test_commands();
        let today_cmd = cmds.iter().find(|c| c.id == "today").unwrap();
        let ctx = SlashContext {
            content: "prefix ".into(),
            cursor_offset: 7,
            block_id: "test".into(),
            marker: None,
            priority: None,
        };
        let action = (today_cmd.execute)(&ctx);
        match action {
            SlashCommandAction::UpdateContent { content, cursor } => {
                assert!(
                    content.contains("[["),
                    "Today command should insert [[date]]"
                );
                assert!(
                    content.starts_with("prefix "),
                    "Should preserve content before cursor"
                );
                assert!(
                    cursor > 7,
                    "Cursor should move past inserted text"
                );
            }
            _ => panic!("Expected UpdateContent action"),
        }
    }

    #[test]
    fn test_heading_command_strips_existing() {
        let cmds = make_test_commands();
        let h1_cmd = cmds.iter().find(|c| c.id == "h1").unwrap();
        let ctx = SlashContext {
            content: "### sub text".into(),
            cursor_offset: 11,
            block_id: "test".into(),
            marker: None,
            priority: None,
        };
        let action = (h1_cmd.execute)(&ctx);
        match action {
            SlashCommandAction::UpdateContent { content, .. } => {
                assert_eq!(content, "# sub text", "Should replace ### with #");
            }
            _ => panic!("Expected UpdateContent action"),
        }
    }

    #[test]
    fn test_normal_text_removes_heading() {
        let cmds = make_test_commands();
        let normal_cmd = cmds.iter().find(|c| c.id == "normal-text").unwrap();
        let ctx = SlashContext {
            content: "## hello".into(),
            cursor_offset: 8,
            block_id: "test".into(),
            marker: None,
            priority: None,
        };
        let action = (normal_cmd.execute)(&ctx);
        match action {
            SlashCommandAction::UpdateContent { content, .. } => {
                assert_eq!(content, "hello", "Should strip ## prefix");
            }
            _ => panic!("Expected UpdateContent action"),
        }
    }

    #[test]
    fn test_strip_heading_variants() {
        assert_eq!(strip_heading("# foo"), "foo");
        assert_eq!(strip_heading("## foo"), "foo");
        assert_eq!(strip_heading("### foo"), "foo");
        assert_eq!(strip_heading("plain"), "plain");
        assert_eq!(strip_heading("  # spaced"), "spaced");
    }

    #[test]
    fn test_strip_marker_variants() {
        assert_eq!(strip_marker("TODO hello"), "hello");
        assert_eq!(strip_marker("DOING hello"), "hello");
        assert_eq!(strip_marker("DONE hello"), "hello");
        assert_eq!(strip_marker("done hello"), "hello");
        assert_eq!(strip_marker("plain"), "plain");
    }
}
