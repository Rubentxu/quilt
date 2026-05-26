//! Page-level outliner coordinator.
//!
//! Owns the undo/redo history and mediates content updates.
//! The `PageOutliner` wraps a `HistoryStack` and applies content
//! changes through a caller-provided callback, allowing it to work
//! with any state management (Leptos signals, mock stores in tests).
//!
//! # Current scope (Batch 9)
//!
//! - Content changes (`SetContent`, `AutocompleteInsert`) are recorded
//!   and can be undone/redone.
//! - Structural operations (indent, outdent, split, merge) are integrated
//!   via `record_structural` and `structural_apply`.

use crate::outliner::history::{build_content_command, invert_command, HistoryStack, OutlinerCommand};
use std::sync::Arc;

/// Error returned by `PageOutliner` when an operation is not supported yet.
#[derive(Debug, Clone, PartialEq)]
pub enum OutlinerError {
    NoUndo,
    NoRedo,
    UnsupportedCommand(String),
}

/// Coordinator for page-level outliner operations.
///
/// # Design
///
/// - Owns a `HistoryStack` for undo/redo.
/// - Content changes push commands to the stack and apply via a caller-provided
///   `apply` callback (e.g., a Leptos `WriteSignal::update`).
/// - Supports both manual content changes (`SetContent`) and autocomplete
///   insertions (`AutocompleteInsert`).
/// - Thread-safe via `Arc<Mutex<...>>`; safe in both WASM and native test targets.
#[derive(Clone)]
pub struct PageOutliner {
    inner: Arc<std::sync::Mutex<PageOutlinerInner>>,
}

struct PageOutlinerInner {
    history: HistoryStack,
    apply: Arc<dyn Fn(&str, &str) + Send + Sync + 'static>,
    /// Callback for structural operations (split, merge, indent, outdent, etc.).
    /// Receives the command to apply. A no-op (noop) callback means structural
    /// operations are not wired to external state.
    structural_apply: Arc<dyn Fn(&OutlinerCommand) + Send + Sync + 'static>,
}

impl PageOutliner {
    /// Create a new page outliner.
    ///
    /// - `capacity`: maximum number of history entries.
    /// - `apply`: callback invoked with `(block_id, new_content)` to apply
    ///   a content change to external state (e.g., a block list signal).
    pub fn new<F>(capacity: usize, apply: F) -> Self
    where
        F: Fn(&str, &str) + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(std::sync::Mutex::new(PageOutlinerInner {
                history: HistoryStack::new(capacity),
                apply: Arc::new(apply),
                structural_apply: Arc::new(|_| {}), // no-op by default
            })),
        }
    }

    /// Create a new page outliner with both a content apply callback
    /// and a structural apply callback.
    ///
    /// - `capacity`: maximum number of history entries.
    /// - `apply`: callback invoked with `(block_id, new_content)` for content changes.
    /// - `structural_apply`: callback invoked with an `OutlinerCommand` for structural
    ///   operations (split, merge, indent, outdent). The caller is responsible for
    ///   mutating the block list to reflect the command.
    pub fn new_with_structural<F, G>(
        capacity: usize,
        apply: F,
        structural_apply: G,
    ) -> Self
    where
        F: Fn(&str, &str) + Send + Sync + 'static,
        G: Fn(&OutlinerCommand) + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(std::sync::Mutex::new(PageOutlinerInner {
                history: HistoryStack::new(capacity),
                apply: Arc::new(apply),
                structural_apply: Arc::new(structural_apply),
            })),
        }
    }

    /// Record a structural command and apply it.
    ///
    /// The command is pushed to the undo history and dispatched to the
    /// `structural_apply` callback. The callback is responsible for mutating
    /// the external block state (e.g., inserting a new block for SplitBlock,
    /// removing a block for MergeBlock). The same callback is used for undo/redo.
    ///
    /// Supported structural command types: `SplitBlock`, `MergeBlock`,
    /// `Indent`, `Outdent`.
    pub fn record_structural(&self, cmd: OutlinerCommand) {
        let structural = {
            let mut inner = self.inner.lock().expect("PageOutliner lock");
            inner.history.push(cmd.clone());
            inner.structural_apply.clone()
        };
        structural(&cmd);
    }

    /// Record a content change and apply it.
    ///
    /// - `trigger`: `None` for manual edits, `Some("page")`/`Some("tag")`/etc.
    ///   for autocomplete insertions.
    pub fn record_content_change(
        &self,
        block_id: &str,
        before: &str,
        after: &str,
        trigger: Option<&str>,
    ) {
        let cmd = build_content_command(block_id, before, after, trigger);
        let apply = {
            let mut inner = self.inner.lock().expect("PageOutliner lock");
            inner.history.push(cmd);
            inner.apply.clone()
        };
        apply(block_id, after);
    }

    /// Undo the last change (content or structural).
    ///
    /// Returns `true` if there was something to undo, `false` otherwise.
    pub fn undo(&self) -> bool {
        let entry = {
            let mut inner = self.inner.lock().expect("PageOutliner lock");
            let cmd = match inner.history.undo() {
                Some(c) => c,
                None => return false,
            };
            let inverse = invert_command(&cmd);
            let apply = inner.apply.clone();
            let structural = inner.structural_apply.clone();
            (inverse, apply, structural)
        };
        let (inverse, apply, structural) = entry;
        dispatch_undo_redo(&inverse, &*apply, &*structural)
    }

    /// Redo the last undone change (content or structural).
    ///
    /// Returns `true` if there was something to redo, `false` otherwise.
    pub fn redo(&self) -> bool {
        let entry = {
            let mut inner = self.inner.lock().expect("PageOutliner lock");
            let cmd = match inner.history.redo() {
                Some(c) => c,
                None => return false,
            };
            let apply = inner.apply.clone();
            let structural = inner.structural_apply.clone();
            (cmd, apply, structural)
        };
        let (cmd, apply, structural) = entry;
        dispatch_undo_redo(&cmd, &*apply, &*structural)
    }

    /// Returns `true` if there are commands to undo.
    pub fn can_undo(&self) -> bool {
        self.inner
            .lock()
            .expect("PageOutliner lock")
            .history
            .can_undo()
    }

    /// Returns `true` if there are commands to redo.
    pub fn can_redo(&self) -> bool {
        self.inner
            .lock()
            .expect("PageOutliner lock")
            .history
            .can_redo()
    }
}

/// Dispatch a command for undo/redo.
///
/// Content-type commands (`SetContent`, `AutocompleteInsert`) are applied
/// via the content `apply` callback. Structural-type commands are dispatched
/// via the `structural_apply` callback.
///
/// Returns `true` if the command was handled by either callback.
fn dispatch_undo_redo(
    cmd: &OutlinerCommand,
    apply: &dyn Fn(&str, &str),
    structural: &dyn Fn(&OutlinerCommand),
) -> bool {
    match cmd {
        OutlinerCommand::SetContent { block_id, after, .. }
        | OutlinerCommand::AutocompleteInsert { block_id, after, .. } => {
            apply(block_id, after);
            true
        }
        // All other command types are structural
        _ => {
            structural(cmd);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Helper: create a PageOutliner that records all `apply` calls.
    fn make_recording_outliner() -> (PageOutliner, Arc<Mutex<Vec<(String, String)>>>) {
        let recorded: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let rec = recorded.clone();
        let apply = move |block_id: &str, content: &str| {
            rec.lock()
                .unwrap()
                .push((block_id.to_string(), content.to_string()));
        };
        let outliner = PageOutliner::new(100, apply);
        (outliner, recorded)
    }

    /// Helper: create a recording outliner with a structural callback.
    /// Uses `Arc<std::sync::Mutex>` for recording (thread-safe, standard Rust).
    fn make_structural_outliner(
    ) -> (
        PageOutliner,
        Arc<Mutex<Vec<(String, String)>>>,
        Arc<Mutex<Vec<OutlinerCommand>>>,
    ) {
        let recorded_content: Arc<Mutex<Vec<(String, String)>>> =
            Arc::new(Mutex::new(Vec::new()));
        let recorded_struct: Arc<Mutex<Vec<OutlinerCommand>>> =
            Arc::new(Mutex::new(Vec::new()));

        let rc = recorded_content.clone();
        let content_apply = move |block_id: &str, content: &str| {
            rc.lock()
                .unwrap()
                .push((block_id.to_string(), content.to_string()));
        };

        let rs = recorded_struct.clone();
        let struct_apply = move |cmd: &OutlinerCommand| {
            rs.lock().unwrap().push(cmd.clone());
        };

        let outliner = PageOutliner::new_with_structural(100, content_apply, struct_apply);
        (outliner, recorded_content, recorded_struct)
    }

    /// Helper: simple notifier using an `AtomicU64` counter.
    /// Avoids potential Mutex issues for simple undo/redo counting.
    fn make_counting_outliner() -> (PageOutliner, Arc<std::sync::atomic::AtomicU64>) {
        let content_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let structural_count = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let cc = content_count.clone();
        let content_apply = move |_: &str, _: &str| {
            cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        };

        let sc = structural_count.clone();
        let struct_apply = move |_: &OutlinerCommand| {
            sc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        };

        let outliner = PageOutliner::new_with_structural(100, content_apply, struct_apply);
        (outliner, content_count)
    }

    // ── RED: Empty outliner ──

    #[test]
    fn test_new_outliner_has_no_undo_redo() {
        let recorded: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let apply = {
            let r = recorded.clone();
            move |_: &str, _: &str| {
                r.lock()
                    .unwrap()
                    .push(("called".to_string(), String::new()));
            }
        };
        let outliner = PageOutliner::new(100, apply);
        assert!(
            !outliner.can_undo(),
            "New outliner should have nothing to undo"
        );
        assert!(
            !outliner.can_redo(),
            "New outliner should have nothing to redo"
        );
        assert!(!outliner.undo(), "undo on empty returns false");
        assert!(!outliner.redo(), "redo on empty returns false");
    }

    // ── RED: Record and undo ──

    #[test]
    fn test_record_then_undo() {
        let (outliner, recorded) = make_recording_outliner();

        outliner.record_content_change("b1", "old", "new", None);
        assert!(outliner.can_undo(), "After record, can undo");
        assert!(!outliner.can_redo(), "After record, nothing to redo");
        assert_eq!(
            recorded.lock().unwrap().len(),
            1,
            "apply called once on record"
        );
        assert_eq!(recorded.lock().unwrap()[0].0, "b1");
        assert_eq!(recorded.lock().unwrap()[0].1, "new");

        recorded.lock().unwrap().clear();
        let ok = outliner.undo();
        assert!(ok, "undo should succeed");
        assert_eq!(
            recorded.lock().unwrap().len(),
            1,
            "undo calls apply once"
        );
        assert_eq!(recorded.lock().unwrap()[0].1, "old", "undo restores old");
        assert!(!outliner.can_undo(), "After undo, nothing to undo");
        assert!(outliner.can_redo(), "After undo, can redo");
    }

    // ── RED: Undo/redo cycle ──

    #[test]
    fn test_undo_redo_cycle() {
        let (outliner, recorded) = make_recording_outliner();

        outliner.record_content_change("b1", "hello", "world", None);
        recorded.lock().unwrap().clear();

        // Undo restores "hello"
        assert!(outliner.undo());
        assert_eq!(recorded.lock().unwrap()[0].1, "hello");

        recorded.lock().unwrap().clear();

        // Redo re-applies "world"
        assert!(outliner.redo());
        assert_eq!(recorded.lock().unwrap()[0].1, "world");

        assert!(outliner.can_undo(), "After redo, can undo again");
        assert!(!outliner.can_redo(), "After redo, nothing more to redo");
    }

    // ── RED: Multiple records undo in reverse ──

    #[test]
    fn test_multiple_records_undo_reverse() {
        let (outliner, recorded) = make_recording_outliner();

        outliner.record_content_change("b1", "a", "b", None);
        outliner.record_content_change("b1", "b", "c", None);

        // Undo should reverse order: last change first
        recorded.lock().unwrap().clear();
        assert!(outliner.undo());
        assert_eq!(recorded.lock().unwrap()[0].1, "b");

        recorded.lock().unwrap().clear();
        assert!(outliner.undo());
        assert_eq!(recorded.lock().unwrap()[0].1, "a");
    }

    // ── RED: Record after undo truncates redo ──

    #[test]
    fn test_record_after_undo_truncates_redo() {
        let (outliner, recorded) = make_recording_outliner();

        outliner.record_content_change("b1", "a", "b", None);
        outliner.record_content_change("b1", "b", "c", None);

        outliner.undo(); // back to "b"
        assert!(
            outliner.can_redo(),
            "Should have something to redo after undo"
        );

        recorded.lock().unwrap().clear();
        outliner.record_content_change("b1", "b", "x", None);

        assert!(
            !outliner.can_redo(),
            "New record should truncate redo buffer"
        );
        assert!(outliner.can_undo(), "Should still have commands to undo");
        assert_eq!(recorded.lock().unwrap()[0].1, "x");
    }

    // ── GREEN: Autocomplete insert roundtrip ──

    #[test]
    fn test_autocomplete_insert_undo_redo() {
        let (outliner, recorded) = make_recording_outliner();

        outliner.record_content_change(
            "b1",
            "see [[proj",
            "see [[Project Alpha]]",
            Some("page"),
        );

        assert!(outliner.can_undo());
        assert_eq!(recorded.lock().unwrap()[0].1, "see [[Project Alpha]]");

        recorded.lock().unwrap().clear();
        assert!(outliner.undo());
        assert_eq!(recorded.lock().unwrap()[0].1, "see [[proj");

        recorded.lock().unwrap().clear();
        assert!(outliner.redo());
        assert_eq!(recorded.lock().unwrap()[0].1, "see [[Project Alpha]]");
    }

    // ── GREEN: can_undo/can_redo tracking ──

    #[test]
    fn test_can_undo_redo_tracking() {
        let (outliner, _) = make_recording_outliner();
        assert!(!outliner.can_undo());
        assert!(!outliner.can_redo());

        outliner.record_content_change("b1", "a", "b", None);
        assert!(outliner.can_undo());
        assert!(!outliner.can_redo());

        outliner.undo();
        assert!(!outliner.can_undo());
        assert!(outliner.can_redo());

        outliner.redo();
        assert!(outliner.can_undo());
        assert!(!outliner.can_redo());
    }

    // ── GREEN: Double undo returns false ──

    #[test]
    fn test_double_undo_returns_false() {
        let (outliner, _) = make_recording_outliner();
        outliner.record_content_change("b1", "a", "b", None);
        assert!(outliner.undo());
        assert!(!outliner.undo(), "Second undo should return false");
    }

    // ── INTEGRATION: PageOutliner + real autocomplete pipeline ──

    #[test]
    fn test_page_outliner_with_autocomplete_pipeline() {
        use crate::parser::autocomplete::{
            AutocompleteCategory, AutocompleteItem, AutocompleteTrigger,
        };
        use crate::parser::autocomplete_pipeline::compute_insertion;

        let content = "see [[proj";
        let trigger = AutocompleteTrigger::PageRef {
            prefix: "proj".into(),
        };
        let item = AutocompleteItem {
            label: "Project Alpha".into(),
            insert_text: "Project Alpha".into(),
            description: None,
            category: AutocompleteCategory::Page,
        };

        let result =
            compute_insertion(content, &trigger, &item).expect("autocomplete insertion");

        let (outliner, recorded) = make_recording_outliner();

        // Record the autocomplete insertion
        outliner.record_content_change("b1", content, &result.new_content, Some("page"));

        // Verify initial application
        assert_eq!(recorded.lock().unwrap()[0].1, "see [[Project Alpha]]");

        // Undo
        recorded.lock().unwrap().clear();
        assert!(outliner.undo());
        assert_eq!(recorded.lock().unwrap()[0].1, "see [[proj");

        // Redo
        recorded.lock().unwrap().clear();
        assert!(outliner.redo());
        assert_eq!(recorded.lock().unwrap()[0].1, "see [[Project Alpha]]");
    }

    // ═══════════════════════════════════════════════════════════════
    //  BATCH 7 — Structural operations through PageOutliner
    // ═══════════════════════════════════════════════════════════════

    // ── RED: Record structural command with no structural callback ──
    // When no structural callback is configured, record_structural
    // should still push the command to history but not apply it.

    #[test]
    fn test_record_structural_without_structural_callback() {
        let (outliner, _) = make_recording_outliner();

        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };

        outliner.record_structural(cmd.clone());

        assert!(outliner.can_undo(), "After structural record, can undo");
        assert!(
            !outliner.can_redo(),
            "After structural record, nothing to redo"
        );
    }

    // ── RED: Record structural command calls structural callback ──

    #[test]
    fn test_record_structural_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };

        outliner.record_structural(cmd.clone());

        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(
            struct_calls.len(),
            1,
            "structural_apply called once on record"
        );
        match &struct_calls[0] {
            OutlinerCommand::SplitBlock { block_id, .. } => {
                assert_eq!(block_id, "b1");
            }
            other => panic!("Expected SplitBlock, got {:?}", other),
        }
    }

    // ── RED: Undo structural command calls structural callback with inverse ──

    #[test]
    fn test_undo_structural_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };

        outliner.record_structural(cmd);
        recorded_struct.lock().unwrap().clear();

        // Undo should call structural_apply with the inverse (MergeBlock)
        assert!(outliner.undo(), "Undo of structural command should succeed");

        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "undo calls structural_apply once");
        match &struct_calls[0] {
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
            other => panic!("Expected MergeBlock (inverse of Split), got {:?}", other),
        }
    }

    // ── RED: Redo structural command calls structural callback ──

    #[test]
    fn test_redo_structural_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };

        outliner.record_structural(cmd.clone());
        recorded_struct.lock().unwrap().clear();

        // Undo first
        assert!(outliner.undo());
        recorded_struct.lock().unwrap().clear();

        // Then redo should re-apply the original SplitBlock
        assert!(outliner.redo(), "Redo should succeed");
        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "redo calls structural_apply once");
        match &struct_calls[0] {
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
            other => panic!("Expected SplitBlock (redo), got {:?}", other),
        }
    }

    // ── GREEN: Structural + content commands interleaved ──

    #[test]
    fn test_interleaved_content_and_structural_commands() {
        let (outliner, recorded_content, recorded_struct) = make_structural_outliner();

        // Use explicit MutexGuard variables to ensure proper lifetime management
        // and avoid any potential issues with temporary MutexGuards.

        // 1. Content change "old" → "new"
        outliner.record_content_change("b1", "old", "new", None);
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard.len(), 1);
            // guard dropped at end of block
        }

        // 2. Split block at cursor
        let split = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hel".into(),
            second_part: "lo".into(),
        };
        outliner.record_structural(split);
        {
            let guard = recorded_struct.lock().unwrap();
            assert_eq!(guard.len(), 1);
        }

        // 3. Second content change "lo" → "lo world"
        outliner.record_content_change("b2", "lo", "lo world", None);
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard.len(), 2);
        }

        // Undo last content change (should restore "lo")
        {
            let mut guard = recorded_content.lock().unwrap();
            guard.clear();
        }
        assert!(outliner.undo(), "Should undo content change");
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(
                guard[0].1, "lo",
                "Content undo restores 'lo'"
            );
        }

        // Undo structural (split → merge)
        {
            let mut guard = recorded_struct.lock().unwrap();
            guard.clear();
        }
        assert!(outliner.undo(), "Should undo structural split");
        {
            let guard = recorded_struct.lock().unwrap();
            assert_eq!(guard.len(), 1, "undo calls structural_apply once");
            match &guard[0] {
                OutlinerCommand::MergeBlock { target_id, .. } => {
                    assert_eq!(target_id, "b1");
                }
                other => panic!("Expected MergeBlock, got {:?}", other),
            }
        }

        // Undo first content change
        {
            let mut guard = recorded_content.lock().unwrap();
            guard.clear();
        }
        assert!(outliner.undo(), "Should undo first content change");
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard[0].1, "old");
        }

        // Redo all three in order
        {
            let mut guard = recorded_content.lock().unwrap();
            guard.clear();
        }
        assert!(outliner.redo(), "Redo first content");
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard[0].1, "new");
        }

        {
            let mut guard = recorded_struct.lock().unwrap();
            guard.clear();
        }
        assert!(outliner.redo(), "Redo split");
        {
            let guard = recorded_struct.lock().unwrap();
            match &guard[0] {
                OutlinerCommand::SplitBlock { block_id, .. } => {
                    assert_eq!(block_id, "b1");
                }
                other => panic!("Expected SplitBlock, got {:?}", other),
            }
        }

        {
            let mut guard = recorded_content.lock().unwrap();
            guard.clear();
        }
        assert!(outliner.redo(), "Redo second content");
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard[0].1, "lo world");
        }
    }

    /// Variant using atomic counters to side-step any potential Mutex issue.
    #[test]
    fn test_interleaved_atomic() {
        let (outliner, content_count) = make_counting_outliner();

        // 1. Content change
        outliner.record_content_change("b1", "old", "new", None);
        assert_eq!(content_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // 2. Split
        let split = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hel".into(),
            second_part: "lo".into(),
        };
        outliner.record_structural(split);

        // 3. Second content change
        outliner.record_content_change("b2", "lo", "lo world", None);
        assert_eq!(content_count.load(std::sync::atomic::Ordering::SeqCst), 2);

        // Undo last (content change)
        assert!(outliner.undo(), "Should undo content change");

        // Undo structural
        assert!(outliner.undo(), "Should undo structural split");

        // Undo first content change
        assert!(outliner.undo(), "Should undo first content change");

        // Now redo all three in order
        assert!(outliner.redo(), "Redo first content");
        assert!(outliner.redo(), "Redo split");
        assert!(outliner.redo(), "Redo second content");
    }

    // ── TRIANGULATE: Undo/redo after structural truncation ──

    #[test]
    fn test_content_after_structural_truncates_structural_redo() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd1 = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "A".into(),
            second_part: "B".into(),
        };
        let cmd2 = OutlinerCommand::SplitBlock {
            block_id: "b2".into(),
            new_block_id: "b3".into(),
            first_part: "B".into(),
            second_part: "C".into(),
        };

        outliner.record_structural(cmd1);
        outliner.record_structural(cmd2);

        // Undo one structural
        recorded_struct.lock().unwrap().clear();
        assert!(outliner.undo());
        // Now push a CONTENT change — should truncate the structural redo
        outliner.record_content_change("b1", "x", "y", None);

        assert!(
            !outliner.can_redo(),
            "Content change after undo truncates structural redo"
        );
        assert!(
            outliner.can_undo(),
            "Structural + content changes remain for undo"
        );
    }

    // ── TRIANGULATE: Can undo structural, then push structural, truncates ──

    #[test]
    fn test_structural_after_undo_truncates_structural_redo() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd1 = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };
        let cmd2 = OutlinerCommand::SplitBlock {
            block_id: "b2".into(),
            new_block_id: "b3".into(),
            first_part: " World".into(),
            second_part: "!".into(),
        };

        outliner.record_structural(cmd1);
        outliner.record_structural(cmd2);

        // Undo one
        recorded_struct.lock().unwrap().clear();
        assert!(outliner.undo());
        assert!(outliner.can_redo(), "Before push, redo should be available");

        // Push new structural command — truncates
        let cmd3 = OutlinerCommand::MergeBlock {
            target_id: "b1".into(),
            source_id: "b2".into(),
            target_before: "Hello World".into(),
            source_before: "!".into(),
        };
        recorded_struct.lock().unwrap().clear();
        outliner.record_structural(cmd3);

        assert!(
            !outliner.can_redo(),
            "New structural command truncates redo"
        );
        assert!(outliner.can_undo(), "Should still have undoable commands");
    }

    // ── TRIANGULATE: Record structural on empty outliner ──

    #[test]
    fn test_structural_can_undo_redo_tracking() {
        let (outliner, _, _) = make_structural_outliner();

        assert!(!outliner.can_undo(), "Initially no undo");
        assert!(!outliner.can_redo(), "Initially no redo");

        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "A".into(),
            second_part: "B".into(),
        };

        outliner.record_structural(cmd);
        assert!(outliner.can_undo(), "After structural, can undo");
        assert!(!outliner.can_redo(), "After structural, no redo");

        outliner.undo();
        assert!(!outliner.can_undo(), "After undo, nothing to undo");
        assert!(outliner.can_redo(), "After undo, can redo");

        outliner.redo();
        assert!(outliner.can_undo(), "After redo, can undo again");
        assert!(!outliner.can_redo(), "After redo, nothing more to redo");
    }

    // ── TRIANGULATE: Structural undo returns false when nothing to undo ──

    #[test]
    fn test_double_undo_structural_returns_false() {
        let (outliner, _, _) = make_structural_outliner();

        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "A".into(),
            second_part: "B".into(),
        };

        outliner.record_structural(cmd);
        assert!(outliner.undo(), "First undo should succeed");
        assert!(
            !outliner.undo(),
            "Second undo (no more undoable) should return false"
        );
    }

    // ═══════════════════════════════════════════════════════════════
    //  BATCH 8 — Indent/Outdent through PageOutliner
    // ═══════════════════════════════════════════════════════════════

    // ── RED: Record indent command calls structural apply ──

    #[test]
    fn test_record_indent_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 2.001,
        };

        outliner.record_structural(cmd.clone());

        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "structural_apply called once on record");
        match &struct_calls[0] {
            OutlinerCommand::Indent { block_id, .. } => {
                assert_eq!(block_id, "b2");
            }
            other => panic!("Expected Indent, got {:?}", other),
        }
    }

    // ── RED: Record outdent command calls structural apply ──

    #[test]
    fn test_record_outdent_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: None,
            new_order: 2.0,
        };

        outliner.record_structural(cmd.clone());

        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "structural_apply called once on record");
        match &struct_calls[0] {
            OutlinerCommand::Outdent { block_id, .. } => {
                assert_eq!(block_id, "b2");
            }
            other => panic!("Expected Outdent, got {:?}", other),
        }
    }

    // ── RED: Undo indent calls structural apply with swapped parent/order ──

    #[test]
    fn test_undo_indent_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 2.001,
        };

        outliner.record_structural(cmd);
        recorded_struct.lock().unwrap().clear();

        // Undo should call structural_apply with the inverse (swapped old/new)
        assert!(outliner.undo(), "Undo of indent should succeed");

        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "undo calls structural_apply once");
        match &struct_calls[0] {
            OutlinerCommand::Indent {
                block_id,
                old_parent,
                old_order,
                new_parent,
                new_order,
            } => {
                assert_eq!(block_id, "b2");
                // old/new should be swapped in the inverse
                assert_eq!(old_parent.as_deref(), Some("b1"),
                    "undo inverse: old_parent should be the former new_parent");
                assert!((old_order - 2.001).abs() < f64::EPSILON,
                    "undo inverse: old_order should be the former new_order");
                assert!(new_parent.is_none(),
                    "undo inverse: new_parent should be the former old_parent (None)");
                assert!((new_order - 2.0).abs() < f64::EPSILON,
                    "undo inverse: new_order should be the former old_order");
            }
            other => panic!("Expected Indent (inverse), got {:?}", other),
        }
    }

    // ── RED: Undo outdent calls structural apply with swapped parent/order ──

    #[test]
    fn test_undo_outdent_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: None,
            new_order: 2.0,
        };

        outliner.record_structural(cmd);
        recorded_struct.lock().unwrap().clear();

        assert!(outliner.undo(), "Undo of outdent should succeed");

        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "undo calls structural_apply once");
        match &struct_calls[0] {
            OutlinerCommand::Outdent {
                block_id,
                old_parent,
                old_order,
                new_parent,
                new_order,
            } => {
                assert_eq!(block_id, "b2");
                // old/new should be swapped in the inverse
                assert!(old_parent.is_none(),
                    "undo inverse: old_parent should be the former new_parent (None)");
                assert!((old_order - 2.0).abs() < f64::EPSILON,
                    "undo inverse: old_order should be the former new_order");
                assert_eq!(new_parent.as_deref(), Some("b1"),
                    "undo inverse: new_parent should be the former old_parent");
                assert!((new_order - 1.5).abs() < f64::EPSILON,
                    "undo inverse: new_order should be the former old_order");
            }
            other => panic!("Expected Outdent (inverse), got {:?}", other),
        }
    }

    // ── RED: Redo indent re-applies the original command ──

    #[test]
    fn test_redo_indent_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 2.001,
        };

        outliner.record_structural(cmd.clone());
        recorded_struct.lock().unwrap().clear();

        // Undo first
        assert!(outliner.undo());
        recorded_struct.lock().unwrap().clear();

        // Then redo should re-apply the original Indent
        assert!(outliner.redo(), "Redo should succeed");
        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "redo calls structural_apply once");
        match &struct_calls[0] {
            OutlinerCommand::Indent {
                block_id,
                old_parent,
                old_order,
                new_parent,
                new_order,
            } => {
                assert_eq!(block_id, "b2");
                assert!(old_parent.is_none());
                assert!((old_order - 2.0).abs() < f64::EPSILON);
                assert_eq!(new_parent.as_deref(), Some("b1"));
                assert!((new_order - 2.001).abs() < f64::EPSILON);
            }
            other => panic!("Expected Indent (redo), got {:?}", other),
        }
    }

    // ── RED: Redo outdent re-applies the original command ──

    #[test]
    fn test_redo_outdent_calls_structural_apply() {
        let (outliner, _, recorded_struct) = make_structural_outliner();

        let cmd = OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: None,
            new_order: 2.0,
        };

        outliner.record_structural(cmd.clone());
        recorded_struct.lock().unwrap().clear();

        // Undo first
        assert!(outliner.undo());
        recorded_struct.lock().unwrap().clear();

        // Then redo should re-apply the original Outdent
        assert!(outliner.redo(), "Redo should succeed");
        let struct_calls = recorded_struct.lock().unwrap();
        assert_eq!(struct_calls.len(), 1, "redo calls structural_apply once");
        match &struct_calls[0] {
            OutlinerCommand::Outdent {
                block_id,
                old_parent,
                old_order,
                new_parent,
                new_order,
            } => {
                assert_eq!(block_id, "b2");
                assert_eq!(old_parent.as_deref(), Some("b1"));
                assert!((old_order - 1.5).abs() < f64::EPSILON);
                assert!(new_parent.is_none());
                assert!((new_order - 2.0).abs() < f64::EPSILON);
            }
            other => panic!("Expected Outdent (redo), got {:?}", other),
        }
    }

    // ── TRIANGULATE: Indent/Outdent mixed with content ──

    #[test]
    fn test_indent_content_interleaved() {
        let (outliner, recorded_content, recorded_struct) = make_structural_outliner();

        // Content change
        outliner.record_content_change("b1", "old", "new", None);
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard.len(), 1);
        }

        // Indent command
        let indent_cmd = OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 2.001,
        };
        outliner.record_structural(indent_cmd);
        {
            let guard = recorded_struct.lock().unwrap();
            assert_eq!(guard.len(), 1);
        }

        // Outdent command
        let outdent_cmd = OutlinerCommand::Outdent {
            block_id: "b3".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: None,
            new_order: 3.0,
        };
        outliner.record_structural(outdent_cmd);
        {
            let guard = recorded_struct.lock().unwrap();
            assert_eq!(guard.len(), 2);
        }

        // Undo: last is outdent
        {
            let mut guard = recorded_struct.lock().unwrap();
            guard.clear();
            recorded_content.lock().unwrap().clear();
        }
        assert!(outliner.undo(), "Should undo outdent");
        {
            let guard = recorded_struct.lock().unwrap();
            assert_eq!(guard.len(), 1);
            match &guard[0] {
                OutlinerCommand::Outdent { block_id, .. } => {
                    assert_eq!(block_id, "b3");
                }
                other => panic!("Expected Outdent inverse, got {:?}", other),
            }
        }

        // Undo: indent
        {
            recorded_struct.lock().unwrap().clear();
        }
        assert!(outliner.undo(), "Should undo indent");
        {
            let guard = recorded_struct.lock().unwrap();
            assert_eq!(guard.len(), 1);
            match &guard[0] {
                OutlinerCommand::Indent { block_id, .. } => {
                    assert_eq!(block_id, "b2");
                }
                other => panic!("Expected Indent inverse, got {:?}", other),
            }
        }

        // Undo: content
        {
            recorded_content.lock().unwrap().clear();
        }
        assert!(outliner.undo(), "Should undo content");
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard[0].1, "old");
        }

        // Redo all three in order
        {
            recorded_content.lock().unwrap().clear();
        }
        assert!(outliner.redo(), "Redo content");
        {
            let guard = recorded_content.lock().unwrap();
            assert_eq!(guard[0].1, "new");
        }
        {
            recorded_struct.lock().unwrap().clear();
        }
        assert!(outliner.redo(), "Redo indent");
        {
            let guard = recorded_struct.lock().unwrap();
            match &guard[0] {
                OutlinerCommand::Indent { block_id, .. } => {
                    assert_eq!(block_id, "b2");
                }
                other => panic!("Expected Indent, got {:?}", other),
            }
        }
        {
            recorded_struct.lock().unwrap().clear();
        }
        assert!(outliner.redo(), "Redo outdent");
        {
            let guard = recorded_struct.lock().unwrap();
            match &guard[0] {
                OutlinerCommand::Outdent { block_id, .. } => {
                    assert_eq!(block_id, "b3");
                }
                other => panic!("Expected Outdent, got {:?}", other),
            }
        }
    }
}
