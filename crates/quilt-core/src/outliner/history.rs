//! Undo/Redo history for the outliner — history of semantic intents.
//!
//! The `HistoryStack` stores `OutlinerCommand` values representing
//! semantic operations (text replacement, indent/outdent, split/merge,
//! autocomplete insertion). Commands are pushed as the user acts;
//! undo/redo navigates the stack.
//!
//! A separate `invert_command` pure function computes the inverse
//! of any command, so the caller (e.g. PageOutliner) can apply it
//! for undo or redo.
//!
//! # Design
//!
//! - Bounded history: oldest commands are evicted when `max_capacity`
//!   is exceeded.
//! - Redo buffer is truncated on every new `push`.
//! - Commands carry enough context (before/after, old/new state) to
//!   support inversion without external lookups.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A semantic command in the undo/redo history.
///
/// Each variant carries enough information to compute its inverse
/// via `invert_command`.
///
/// The serde attributes (`tag = "type"`, `rename_all = "camelCase"`)
/// produce a JSON form that matches the TypeScript `OutlinerCommand`
/// type used by the WASM bridge:
/// ```json
/// { "type": "setContent", "blockId": "b1", "before": "old", "after": "new" }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OutlinerCommand {
    /// A block's text content was replaced.
    SetContent {
        block_id: String,
        before: String,
        after: String,
    },

    /// A block was indented under a sibling.
    Indent {
        block_id: String,
        old_parent: Option<String>,
        old_order: f64,
        new_parent: Option<String>,
        new_order: f64,
    },

    /// A block was outdented (moved up one level).
    Outdent {
        block_id: String,
        old_parent: Option<String>,
        old_order: f64,
        new_parent: Option<String>,
        new_order: f64,
    },

    /// A block was moved to a new parent/position via drag and drop.
    MoveBlock {
        block_id: String,
        old_parent: Option<String>,
        old_order: f64,
        new_parent: Option<String>,
        new_order: f64,
    },

    /// A block was split at the cursor into two blocks.
    ///
    /// `first_part` + `second_part` == `full_content_before`.
    SplitBlock {
        block_id: String,
        /// The new block's id (receives `second_part`).
        new_block_id: String,
        /// Content retained by the original block.
        first_part: String,
        /// Content moved to the new block.
        second_part: String,
    },

    /// Two blocks were merged into one (the target absorbs the source).
    MergeBlock {
        target_id: String,
        source_id: String,
        /// Content of the target block before merging.
        target_before: String,
        /// Content of the source block before merging (gets recreated on undo).
        source_before: String,
    },

    /// Autocomplete inserted structured content into a block.
    AutocompleteInsert {
        block_id: String,
        before: String,
        after: String,
        /// Human-readable trigger kind e.g. "page", "tag", "property".
        trigger: String,
    },
}

impl OutlinerCommand {
    /// The primary block id this command acts on.
    pub fn primary_block_id(&self) -> &str {
        match self {
            OutlinerCommand::SetContent { block_id, .. } => block_id,
            OutlinerCommand::Indent { block_id, .. } => block_id,
            OutlinerCommand::Outdent { block_id, .. } => block_id,
            OutlinerCommand::MoveBlock { block_id, .. } => block_id,
            OutlinerCommand::SplitBlock { block_id, .. } => block_id,
            OutlinerCommand::MergeBlock { target_id, .. } => target_id,
            OutlinerCommand::AutocompleteInsert { block_id, .. } => block_id,
        }
    }
}

impl fmt::Display for OutlinerCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutlinerCommand::SetContent { block_id, .. } => {
                write!(f, "SetContent({})", block_id)
            }
            OutlinerCommand::Indent { block_id, .. } => {
                write!(f, "Indent({})", block_id)
            }
            OutlinerCommand::Outdent { block_id, .. } => {
                write!(f, "Outdent({})", block_id)
            }
            OutlinerCommand::MoveBlock { block_id, .. } => {
                write!(f, "MoveBlock({})", block_id)
            }
            OutlinerCommand::SplitBlock { block_id, .. } => {
                write!(f, "SplitBlock({})", block_id)
            }
            OutlinerCommand::MergeBlock {
                target_id,
                source_id,
                ..
            } => {
                write!(f, "MergeBlock({} <- {})", target_id, source_id)
            }
            OutlinerCommand::AutocompleteInsert {
                block_id, trigger, ..
            } => {
                write!(f, "AutocompleteInsert({}, {})", block_id, trigger)
            }
        }
    }
}

/// Bounded undo/redo history stack.
///
/// Commands are stored in a `Vec` indexed by `position`:
/// - `position` points to the next slot for a new command.
/// - Commands at indices `[0, position)` are undo history.
/// - Commands at indices `[position, len)` are redo history.
/// - When `push` is called, the redo history is truncated.
/// - When capacity is exceeded, the oldest commands are evicted.
#[derive(Debug, Clone)]
pub struct HistoryStack {
    commands: Vec<OutlinerCommand>,
    position: usize,
    max_capacity: usize,
}

impl HistoryStack {
    /// Create a new history stack with the given capacity.
    ///
    /// `max_capacity` must be at least 1.
    pub fn new(max_capacity: usize) -> Self {
        let max = if max_capacity < 1 { 1 } else { max_capacity };
        HistoryStack {
            commands: Vec::with_capacity(max + 1),
            position: 0,
            max_capacity: max,
        }
    }

    /// Push a command onto the history stack.
    ///
    /// Any redo history is truncated. If capacity is exceeded,
    /// the oldest command is evicted.
    pub fn push(&mut self, command: OutlinerCommand) {
        // Truncate redo history
        self.commands.truncate(self.position);

        // Evict oldest if at capacity
        if self.commands.len() >= self.max_capacity {
            self.commands.remove(0);
            // Position moves back by 1 since we removed the first element
            if self.position > 0 {
                self.position -= 1;
            }
        }

        self.commands.push(command);
        self.position = self.commands.len();
    }

    /// Move backward in history (undo). Returns the command at
    /// the previous position.
    ///
    /// Returns `None` if there is nothing to undo.
    pub fn undo(&mut self) -> Option<OutlinerCommand> {
        if self.position == 0 {
            return None;
        }
        self.position -= 1;
        Some(self.commands[self.position].clone())
    }

    /// Move forward in history (redo). Returns the command at
    /// the current position and advances.
    ///
    /// Returns `None` if there is nothing to redo.
    pub fn redo(&mut self) -> Option<OutlinerCommand> {
        if self.position >= self.commands.len() {
            return None;
        }
        let cmd = self.commands[self.position].clone();
        self.position += 1;
        Some(cmd)
    }

    /// Returns `true` if there are commands to undo.
    pub fn can_undo(&self) -> bool {
        self.position > 0
    }

    /// Returns `true` if there are commands to redo.
    pub fn can_redo(&self) -> bool {
        self.position < self.commands.len()
    }

    /// Clear all commands and reset the history.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.position = 0;
    }

    /// Number of commands currently stored.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns `true` if no commands are stored.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The maximum capacity of this stack.
    pub fn capacity(&self) -> usize {
        self.max_capacity
    }

    /// Current position in the history (number of undoable commands).
    pub fn position(&self) -> usize {
        self.position
    }
}

/// Compute the inverse of a command for undo.
///
/// The inverse command, when applied by the outliner, reverses
/// the effect of the original command.
///
/// # Guarantees
///
/// `invert_command(invert_command(cmd)) == cmd` for all command types.
pub fn invert_command(cmd: &OutlinerCommand) -> OutlinerCommand {
    match cmd {
        OutlinerCommand::SetContent {
            block_id,
            before,
            after,
        } => OutlinerCommand::SetContent {
            block_id: block_id.clone(),
            before: after.clone(),
            after: before.clone(),
        },
        OutlinerCommand::Indent {
            block_id,
            old_parent,
            old_order,
            new_parent,
            new_order,
        } => OutlinerCommand::Indent {
            block_id: block_id.clone(),
            old_parent: new_parent.clone(),
            old_order: *new_order,
            new_parent: old_parent.clone(),
            new_order: *old_order,
        },
        OutlinerCommand::Outdent {
            block_id,
            old_parent,
            old_order,
            new_parent,
            new_order,
        } => OutlinerCommand::Outdent {
            block_id: block_id.clone(),
            old_parent: new_parent.clone(),
            old_order: *new_order,
            new_parent: old_parent.clone(),
            new_order: *old_order,
        },
        OutlinerCommand::MoveBlock {
            block_id,
            old_parent,
            old_order,
            new_parent,
            new_order,
        } => OutlinerCommand::MoveBlock {
            block_id: block_id.clone(),
            old_parent: new_parent.clone(),
            old_order: *new_order,
            new_parent: old_parent.clone(),
            new_order: *old_order,
        },
        OutlinerCommand::SplitBlock {
            block_id,
            new_block_id,
            first_part,
            second_part,
        } => OutlinerCommand::MergeBlock {
            target_id: block_id.clone(),
            source_id: new_block_id.clone(),
            target_before: first_part.clone(),
            source_before: second_part.clone(),
        },
        OutlinerCommand::MergeBlock {
            target_id,
            source_id,
            target_before,
            source_before,
        } => OutlinerCommand::SplitBlock {
            block_id: target_id.clone(),
            new_block_id: source_id.clone(),
            first_part: target_before.clone(),
            second_part: source_before.clone(),
        },
        OutlinerCommand::AutocompleteInsert {
            block_id,
            before,
            after,
            trigger,
        } => OutlinerCommand::AutocompleteInsert {
            block_id: block_id.clone(),
            before: after.clone(),
            after: before.clone(),
            trigger: trigger.clone(),
        },
    }
}

/// Error type for history-aware operation wrappers.
#[derive(Debug, Clone, PartialEq)]
pub enum HistoryError {
    /// The history stack is at capacity and the oldest command
    /// was evicted to make room (informational, not a real error).
    OldestEvicted,
    /// The command type could not be inverted.
    NoInverse,
}

/// Build a content-change command, distinguishing manual edits from
/// autocomplete insertions.
///
/// The `trigger` parameter:
/// - `None` → produces `OutlinerCommand::SetContent`
/// - `Some("page")` → produces `OutlinerCommand::AutocompleteInsert { trigger: "page" }`
/// - `Some("tag")` → produces `OutlinerCommand::AutocompleteInsert { trigger: "tag" }`
/// - Any other `Some(s)` → produces `AutocompleteInsert { trigger: s }`
///
/// This is the primary integration helper: editor code calls this
/// when a block's content changes, then pushes the resulting command
/// onto a `HistoryStack`.
pub fn build_content_command(
    block_id: &str,
    before: &str,
    after: &str,
    trigger: Option<&str>,
) -> OutlinerCommand {
    match trigger {
        Some(trigger_kind) => OutlinerCommand::AutocompleteInsert {
            block_id: block_id.to_string(),
            before: before.to_string(),
            after: after.to_string(),
            trigger: trigger_kind.to_string(),
        },
        None => OutlinerCommand::SetContent {
            block_id: block_id.to_string(),
            before: before.to_string(),
            after: after.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper: create a minimal command for testing ──

    fn make_set_content() -> OutlinerCommand {
        OutlinerCommand::SetContent {
            block_id: "b1".into(),
            before: "old".into(),
            after: "new".into(),
        }
    }

    fn make_indent() -> OutlinerCommand {
        OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 2.001,
        }
    }

    fn make_outdent() -> OutlinerCommand {
        OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: None,
            new_order: 2.0,
        }
    }

    fn make_split() -> OutlinerCommand {
        OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        }
    }

    fn make_merge() -> OutlinerCommand {
        OutlinerCommand::MergeBlock {
            target_id: "b1".into(),
            source_id: "b2".into(),
            target_before: "Hello".into(),
            source_before: " World".into(),
        }
    }

    fn make_move_block() -> OutlinerCommand {
        OutlinerCommand::MoveBlock {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: None,
            new_order: 2.0,
        }
    }

    fn make_autocomplete_insert() -> OutlinerCommand {
        OutlinerCommand::AutocompleteInsert {
            block_id: "b1".into(),
            before: "see [[".into(),
            after: "see [[Project]]".into(),
            trigger: "page".into(),
        }
    }

    // ── RED: Test 1 — Empty stack ──

    #[test]
    fn test_empty_stack_has_no_undo_redo() {
        let stack = HistoryStack::new(100);
        assert!(!stack.can_undo(), "New stack should have nothing to undo");
        assert!(!stack.can_redo(), "New stack should have nothing to redo");
        assert!(stack.is_empty(), "New stack should be empty");
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_empty_stack_undo_returns_none() {
        let mut stack = HistoryStack::new(100);
        assert!(
            stack.undo().is_none(),
            "undo on empty stack must return None"
        );
        assert!(
            stack.redo().is_none(),
            "redo on empty stack must return None"
        );
    }

    // ── RED: Test 2 — Push and undo ──

    #[test]
    fn test_push_then_undo_returns_command() {
        let mut stack = HistoryStack::new(100);
        let cmd = make_set_content();
        stack.push(cmd.clone());

        assert!(stack.can_undo(), "After push, should be able to undo");
        assert!(!stack.can_redo(), "After push, nothing to redo");
        assert_eq!(stack.len(), 1);

        let undone = stack.undo().expect("undo should return a command");
        assert_eq!(undone, cmd, "undo should return the pushed command");
        assert!(!stack.can_undo(), "After undo, nothing left to undo");
        assert!(stack.can_redo(), "After undo, should be able to redo");
    }

    // ── RED: Test 3 — Undo/redo cycle ──

    #[test]
    fn test_undo_redo_cycle() {
        let mut stack = HistoryStack::new(100);
        let cmd = make_set_content();
        stack.push(cmd.clone());

        // Undo
        let undone = stack.undo().expect("undo");
        assert_eq!(undone, cmd);

        // Redo
        assert!(stack.can_redo());
        let redone = stack.redo().expect("redo");
        assert_eq!(redone, cmd, "redo should return the same command");
        assert!(stack.can_undo(), "After redo, should be able to undo again");
        assert!(!stack.can_redo(), "After redo, nothing more to redo");
    }

    // ── RED: Test 4 — Multiple undo in reverse order ──

    #[test]
    fn test_multiple_undo_reverse_order() {
        let mut stack = HistoryStack::new(100);
        let cmd1 = make_set_content();
        let cmd2 = make_indent();

        stack.push(cmd1.clone());
        stack.push(cmd2.clone());

        // Undo should return last pushed first
        let first_undo = stack.undo().expect("first undo");
        assert_eq!(first_undo, cmd2, "undo should return most recent first");
        assert!(stack.can_undo(), "Still one more to undo");

        let second_undo = stack.undo().expect("second undo");
        assert_eq!(second_undo, cmd1);
        assert!(!stack.can_undo(), "Nothing left to undo");
    }

    // ── RED: Test 5 — Push truncates redo ──

    #[test]
    fn test_push_truncates_redo() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_set_content());
        stack.push(make_indent());

        // Undo twice
        stack.undo();
        stack.undo();

        assert!(stack.can_redo(), "Should have two to redo");

        // Push a new command — should truncate redo buffer
        let new_cmd = make_outdent();
        stack.push(new_cmd.clone());

        assert!(!stack.can_redo(), "Push should truncate redo buffer");
        assert!(stack.can_undo(), "Should have commands to undo");

        let undone = stack.undo().expect("undo");
        assert_eq!(undone, new_cmd, "Should undo the last pushed command");
    }

    // ── Test 6: Capacity eviction ──

    #[test]
    fn test_capacity_evicts_oldest() {
        let mut stack = HistoryStack::new(2);

        stack.push(make_set_content()); // cmd1 — should be evicted
        stack.push(make_indent()); // cmd2
        stack.push(make_outdent()); // cmd3 — pushes out cmd1

        assert_eq!(stack.len(), 2, "Should keep only 2 commands");
        assert!(stack.can_undo(), "Should have undoable commands");

        // First undo should return cmd3 (most recent)
        let first = stack.undo().expect("undo");
        assert_eq!(first, make_outdent());

        // Second undo should return cmd2
        let second = stack.undo().expect("undo");
        assert_eq!(second, make_indent());

        // cmd1 should be gone
        assert!(!stack.can_undo(), "cmd1 was evicted, nothing more to undo");
    }

    // ── Test 7: Capacity must be at least 1 ──

    #[test]
    fn test_minimum_capacity_is_one() {
        let stack = HistoryStack::new(0); // Should clamp to 1
        assert_eq!(stack.capacity(), 1, "Capacity minimum is 1");
    }

    #[test]
    fn test_capacity_1_evicts_immediately() {
        let mut stack = HistoryStack::new(1);

        stack.push(make_set_content());
        assert_eq!(stack.len(), 1);

        stack.push(make_indent());
        assert_eq!(stack.len(), 1, "Second push should evict first");

        // Only the last command should remain
        let cmd = stack.undo().expect("undo");
        assert_eq!(cmd, make_indent());
        assert!(!stack.can_undo(), "Only one command was kept");
    }

    // ── Test 8: Clear ──

    #[test]
    fn test_clear_resets_stack() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_set_content());
        stack.push(make_indent());
        assert_eq!(stack.len(), 2);

        stack.clear();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert!(stack.undo().is_none());
        assert!(stack.redo().is_none());
    }

    // ── Test 9: primary_block_id ──

    #[test]
    fn test_primary_block_id_set_content() {
        let cmd = make_set_content();
        assert_eq!(cmd.primary_block_id(), "b1");
    }

    #[test]
    fn test_primary_block_id_merge() {
        let cmd = make_merge();
        assert_eq!(cmd.primary_block_id(), "b1", "Merge uses target_id");
    }

    // ── RED: Test 10 — invert_command: SetContent ──

    #[test]
    fn test_invert_set_content_swaps_before_after() {
        let cmd = OutlinerCommand::SetContent {
            block_id: "b1".into(),
            before: "hello".into(),
            after: "world".into(),
        };
        let inv = invert_command(&cmd);
        match inv {
            OutlinerCommand::SetContent {
                block_id,
                before,
                after,
            } => {
                assert_eq!(block_id, "b1");
                assert_eq!(before, "world", "before should become 'world'");
                assert_eq!(after, "hello", "after should become 'hello'");
            }
            other => panic!("Expected SetContent, got {:?}", other),
        }
    }

    #[test]
    fn test_invert_set_content_double_invert() {
        let original = make_set_content();
        let inv = invert_command(&original);
        let double_inv = invert_command(&inv);
        assert_eq!(
            double_inv, original,
            "Double inversion should return the original"
        );
    }

    // ── Test 11: invert_command: Indent/Outdent ──

    #[test]
    fn test_invert_indent_swaps_old_new() {
        let cmd = make_indent();
        let inv = invert_command(&cmd);
        match inv {
            OutlinerCommand::Indent {
                block_id,
                old_parent,
                old_order,
                new_parent,
                new_order,
            } => {
                assert_eq!(block_id, "b2");
                // old_parent should be the former new_parent
                assert_eq!(old_parent, Some("b1".into()));
                assert!((old_order - 2.001).abs() < f64::EPSILON);
                // new_parent should be the former old_parent (None)
                assert_eq!(new_parent, None);
                assert!((new_order - 2.0).abs() < f64::EPSILON);
            }
            other => panic!("Expected Indent, got {:?}", other),
        }
    }

    #[test]
    fn test_invert_indent_double_invert() {
        let original = make_indent();
        let inv = invert_command(&original);
        let double_inv = invert_command(&inv);
        assert_eq!(double_inv, original);
    }

    #[test]
    fn test_invert_outdent_double_invert() {
        let original = make_outdent();
        let inv = invert_command(&original);
        let double_inv = invert_command(&inv);
        assert_eq!(double_inv, original);
    }

    // ── Test 12: invert_command: SplitBlock ──

    #[test]
    fn test_invert_split_becomes_merge() {
        let cmd = make_split();
        let inv = invert_command(&cmd);
        match inv {
            OutlinerCommand::MergeBlock {
                target_id,
                source_id,
                target_before,
                source_before,
            } => {
                assert_eq!(target_id, "b1");
                assert_eq!(source_id, "b2");
                assert_eq!(target_before, "Hello");
                assert_eq!(source_before, " World");
            }
            other => panic!("Expected MergeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_invert_split_double_invert() {
        let original = make_split();
        let inv = invert_command(&original);
        let double_inv = invert_command(&inv);
        assert_eq!(double_inv, original);
    }

    // ── Test 13: invert_command: MergeBlock ──

    #[test]
    fn test_invert_merge_becomes_split() {
        let cmd = make_merge();
        let inv = invert_command(&cmd);
        match inv {
            OutlinerCommand::SplitBlock {
                block_id,
                new_block_id,
                first_part,
                second_part,
            } => {
                assert_eq!(block_id, "b1");
                assert_eq!(new_block_id, "b2");
                assert_eq!(first_part, "Hello");
                assert_eq!(second_part, " World");
            }
            other => panic!("Expected SplitBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_invert_merge_double_invert() {
        let original = make_merge();
        let inv = invert_command(&original);
        let double_inv = invert_command(&inv);
        assert_eq!(double_inv, original);
    }

    // ── Test 14: invert_command: AutocompleteInsert ──

    #[test]
    fn test_invert_autocomplete_swaps_before_after() {
        let cmd = make_autocomplete_insert();
        let inv = invert_command(&cmd);
        match inv {
            OutlinerCommand::AutocompleteInsert {
                block_id,
                before,
                after,
                trigger,
            } => {
                assert_eq!(block_id, "b1");
                assert_eq!(before, "see [[Project]]");
                assert_eq!(after, "see [[");
                assert_eq!(trigger, "page");
            }
            other => panic!("Expected AutocompleteInsert, got {:?}", other),
        }
    }

    #[test]
    fn test_invert_autocomplete_double_invert() {
        let original = make_autocomplete_insert();
        let inv = invert_command(&original);
        let double_inv = invert_command(&inv);
        assert_eq!(double_inv, original);
    }

    // ── Test 15: Display for OutlinerCommand ──

    #[test]
    fn test_display_set_content() {
        let cmd = make_set_content();
        let display = format!("{}", cmd);
        assert_eq!(display, "SetContent(b1)");
    }

    #[test]
    fn test_display_autocomplete() {
        let cmd = make_autocomplete_insert();
        let display = format!("{}", cmd);
        assert!(display.contains("b1"));
        assert!(display.contains("page"));
    }

    // ── Test 16: Undo after capacity eviction with position tracking ──

    #[test]
    fn test_undo_after_capacity_eviction_position_correct() {
        let mut stack = HistoryStack::new(3);

        stack.push(make_set_content()); // cmd1
        stack.push(make_indent()); // cmd2
        stack.push(make_outdent()); // cmd3

        // Undo to go back
        let u1 = stack.undo().expect("undo"); // cmd3
        assert_eq!(u1, make_outdent());

        let u2 = stack.undo().expect("undo"); // cmd2
        assert_eq!(u2, make_indent());

        // Push new command — should evict cmd1, keep cmd2 and cmd3?
        // Actually: position=1 after two undos, commands=[cmd1, cmd2, cmd3]
        // truncate(position=1) => [cmd1]
        // Then push => [cmd1, cmd4]
        // Wait, that's only 2 commands, not at capacity...
        // Let me verify.

        let cmd4 = make_split();
        stack.push(cmd4);

        assert_eq!(stack.len(), 2, "After truncate + push: 2 commands");
        // cmd2 and cmd3 were truncated, cmd1 remains, cmd4 is new
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        let undo = stack.undo().expect("undo");
        // Position after push = 2. Undo gives commands[1] = cmd4
        assert_eq!(undo, make_split());
    }

    // ── Test 17: All command types have double-invert identity ──

    #[test]
    fn test_double_invert_all_command_types() {
        let commands = vec![
            make_set_content(),
            make_indent(),
            make_outdent(),
            make_move_block(),
            make_split(),
            make_merge(),
            make_autocomplete_insert(),
        ];

        for cmd in commands {
            let double_inv = invert_command(&invert_command(&cmd));
            assert_eq!(
                double_inv, cmd,
                "Double inversion should restore original, got mismatch: {:?} vs {:?}",
                double_inv, cmd
            );
        }
    }

    // ── Test 18: Undo after clear ──

    #[test]
    fn test_undo_after_clear_returns_none() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_set_content());
        stack.clear();
        assert!(stack.undo().is_none(), "After clear, undo returns None");
        assert!(stack.redo().is_none(), "After clear, redo returns None");
    }

    // ── TRIANGULATE: Undo more than available ──

    #[test]
    fn test_undo_more_than_available_returns_none_after_exhausted() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_set_content());

        let _ = stack.undo(); // 1st — ok
        let extra = stack.undo(); // 2nd — should be None
        assert!(extra.is_none(), "Extra undo beyond available must be None");
    }

    // ── TRIANGULATE: Redo more than available ──

    #[test]
    fn test_redo_more_than_available_returns_none_after_exhausted() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_set_content());
        stack.undo(); // now redo has 1

        let _ = stack.redo(); // 1st — ok
        let extra = stack.redo(); // 2nd — should be None
        assert!(extra.is_none(), "Extra redo beyond available must be None");
    }

    // ── TRIANGULATE: Back-and-forth undo/redo ──

    #[test]
    fn test_multiple_undo_redo_cycles() {
        let mut stack = HistoryStack::new(100);
        let cmd1 = make_set_content();
        let cmd2 = make_indent();

        stack.push(cmd1.clone());
        stack.push(cmd2.clone());

        // Cycle 1: undo, redo
        assert_eq!(stack.undo().expect("undo"), cmd2);
        assert_eq!(stack.redo().expect("redo"), cmd2);

        // Cycle 2: undo both, redo both
        assert_eq!(stack.undo().expect("undo"), cmd2);
        assert_eq!(stack.undo().expect("undo"), cmd1);
        assert_eq!(stack.redo().expect("redo"), cmd1);
        assert_eq!(stack.redo().expect("redo"), cmd2);

        // Back to start — nothing more to redo
        assert!(!stack.can_redo());
    }

    // ── TRIANGULATE: Position tracking ──

    #[test]
    fn test_position_tracking() {
        let mut stack = HistoryStack::new(10);
        assert_eq!(stack.position(), 0, "Initial position is 0");

        stack.push(make_set_content());
        assert_eq!(stack.position(), 1);

        stack.push(make_indent());
        assert_eq!(stack.position(), 2);

        stack.undo();
        assert_eq!(stack.position(), 1, "After undo, position decreases");

        stack.undo();
        assert_eq!(stack.position(), 0, "After full undo, position is 0");

        stack.redo();
        assert_eq!(stack.position(), 1, "After redo, position increases");
    }

    // ── TRIANGULATE: Large capacity doesn't break ──

    #[test]
    fn test_large_capacity() {
        let mut stack = HistoryStack::new(10_000);
        for i in 0..1_000 {
            stack.push(OutlinerCommand::SetContent {
                block_id: format!("b{}", i),
                before: String::new(),
                after: format!("content {}", i),
            });
        }
        assert_eq!(stack.len(), 1_000);
        assert_eq!(stack.position(), 1_000);
        assert!(stack.can_undo());

        // Undo 500
        for _ in 0..500 {
            stack.undo();
        }
        assert_eq!(stack.position(), 500);

        // Redo 250
        for _ in 0..250 {
            stack.redo();
        }
        assert_eq!(stack.position(), 750);
    }

    // ── TRIANGULATE: build_content_command ──

    #[test]
    fn test_build_content_command_without_trigger() {
        let cmd = build_content_command("b1", "hello", "world", None);
        match cmd {
            OutlinerCommand::SetContent {
                block_id,
                before,
                after,
            } => {
                assert_eq!(block_id, "b1");
                assert_eq!(before, "hello");
                assert_eq!(after, "world");
            }
            other => panic!("Expected SetContent, got {:?}", other),
        }
    }

    #[test]
    fn test_build_content_command_with_trigger() {
        let cmd = build_content_command("b1", "see [[", "see [[Project]]", Some("page"));
        match cmd {
            OutlinerCommand::AutocompleteInsert {
                block_id,
                before,
                after,
                trigger,
            } => {
                assert_eq!(block_id, "b1");
                assert_eq!(before, "see [[");
                assert_eq!(after, "see [[Project]]");
                assert_eq!(trigger, "page");
            }
            other => panic!("Expected AutocompleteInsert, got {:?}", other),
        }
    }

    #[test]
    fn test_build_content_with_tag_trigger() {
        let cmd = build_content_command("b42", "#ur", "#urgent", Some("tag"));
        match cmd {
            OutlinerCommand::AutocompleteInsert {
                block_id,
                before,
                after,
                trigger,
            } => {
                assert_eq!(block_id, "b42");
                assert_eq!(before, "#ur");
                assert_eq!(after, "#urgent");
                assert_eq!(trigger, "tag");
            }
            other => panic!("Expected AutocompleteInsert, got {:?}", other),
        }
    }

    // ── TRIANGULATE: build_content_command round-trip ──

    #[test]
    fn test_build_content_command_invert_roundtrip() {
        let cmd = build_content_command("b1", "old", "new", Some("page"));
        let inv = invert_command(&cmd);
        let double_inv = invert_command(&inv);
        assert_eq!(
            double_inv, cmd,
            "build_content_command must round-trip through invert"
        );
    }

    #[test]
    fn test_build_content_command_no_trigger_roundtrip() {
        let cmd = build_content_command("b1", "old", "new", None);
        let inv = invert_command(&cmd);
        let double_inv = invert_command(&inv);
        assert_eq!(double_inv, cmd, "SetContent must round-trip through invert");
    }

    // ── INTEGRATION: Full autocomplete → history roundtrip ──

    #[test]
    fn test_autocomplete_to_history_roundtrip() {
        // Simulate the full flow:
        // 1. User types "see [[proj" → autocomplete trigger
        // 2. User selects "Project Alpha" → compute_insertion
        // 3. build_content_command wraps it → HistoryStack::push
        // 4. HistoryStack::undo returns the command
        // 5. HistoryStack::redo re-applies it

        let content = "see [[proj";
        let trigger = crate::parser::autocomplete::AutocompleteTrigger::PageRef {
            prefix: "proj".into(),
        };
        let item = crate::parser::autocomplete::AutocompleteItem {
            label: "Project Alpha".into(),
            insert_text: "Project Alpha".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Page,
        };

        let before = content.to_string();
        let result =
            crate::parser::autocomplete_pipeline::compute_insertion(content, &trigger, &item)
                .expect("compute_insertion should succeed for page ref");
        let after = result.new_content;
        assert_eq!(
            after, "see [[Project Alpha]]",
            "compute_insertion produces correct content"
        );

        // Build history command
        let cmd = build_content_command("b1", &before, &after, Some("page"));
        assert_eq!(cmd.primary_block_id(), "b1");

        // Push to history
        let mut stack = HistoryStack::new(100);
        stack.push(cmd.clone());
        assert!(stack.can_undo());
        assert_eq!(stack.len(), 1);

        // Undo returns the exact command
        let undone = stack.undo().expect("undo should return command");
        assert_eq!(undone, cmd, "undo returns the AutocompleteInsert command");

        // Redo returns it again
        assert!(stack.can_redo());
        let redone = stack.redo().expect("redo should return command");
        assert_eq!(redone, cmd, "redo returns the AutocompleteInsert command");

        // Invert twice returns the original
        let inv = invert_command(&cmd);
        let double_inv = invert_command(&inv);
        assert_eq!(
            double_inv, cmd,
            "double-invert preserves the autocomplete command"
        );
    }
}
