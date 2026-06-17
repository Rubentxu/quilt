//! Global app state — the cross-graph state Quilt persists outside any
//! single Graph Space (ADR-0030 §5).
//!
//! This state is intentionally narrow: a `last_opened_graph` pointer,
//! a bounded list of recent graphs, and the persisted visibility of the
//! right sidebar. Per ADR-0030 §5, it lives outside the graph, so
//! switching between graphs preserves the recents list, the sidebar
//! preference, and (importantly) lets us point at the *last* graph even
//! if it is currently invalid.
//!
//! See also: [`crate::repositories::global_app_state_repository`].

use std::path::PathBuf;

/// Maximum number of recents kept in [`GlobalAppState::recent_graphs`].
///
/// Older entries beyond this cap are dropped. Order is most-recent-first
/// (the head of the list is the most recently opened graph).
pub const RECENTS_CAP: usize = 10;

/// The cross-graph app state.
///
/// Lives outside any single Graph Space; the canonical storage is
/// `~/.local/share/quilt/global.db` (or `XDG_DATA_HOME/quilt/global.db`).
/// On error, in-memory defaults are used so the server never blocks
/// on global state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalAppState {
    /// The last graph the user successfully opened.
    ///
    /// `None` on a fresh install. May point to a path that is no
    /// longer valid — the validator is the source of truth for
    /// "is this graph openable?".
    pub last_opened_graph: Option<PathBuf>,
    /// Most-recent-first list of recent graphs. Bounded by
    /// [`RECENTS_CAP`].
    pub recent_graphs: Vec<PathBuf>,
    /// Persisted visibility of the contextual right sidebar.
    /// `None` means "no preference expressed yet — fall back to
    /// default (visible on desktop, hidden on mobile)".
    pub right_sidebar_visible: Option<bool>,
}

impl Default for GlobalAppState {
    fn default() -> Self {
        Self {
            last_opened_graph: None,
            recent_graphs: Vec::new(),
            right_sidebar_visible: None,
        }
    }
}

impl GlobalAppState {
    /// Construct from raw parts, enforcing the [`RECENTS_CAP`]
    /// invariant (most-recent-first, dedup).
    pub fn new(
        last_opened_graph: Option<PathBuf>,
        recent_graphs: Vec<PathBuf>,
        right_sidebar_visible: Option<bool>,
    ) -> Self {
        let mut state = Self {
            last_opened_graph,
            recent_graphs: Vec::new(),
            right_sidebar_visible,
        };
        for g in recent_graphs {
            state.push_recent(g);
        }
        state
    }

    /// Push a graph to the head of the recents list, deduped and
    /// bounded to [`RECENTS_CAP`]. Most-recent-first.
    pub fn push_recent(&mut self, path: PathBuf) {
        // Remove any prior occurrence (case-sensitive) so that the
        // push moves the path to the head, not duplicates it.
        self.recent_graphs.retain(|p| p != &path);
        self.recent_graphs.insert(0, path);
        // Bound the list.
        self.recent_graphs.truncate(RECENTS_CAP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_recent_dedupes() {
        let mut s = GlobalAppState::default();
        s.push_recent(PathBuf::from("/a"));
        s.push_recent(PathBuf::from("/b"));
        s.push_recent(PathBuf::from("/a")); // re-push moves /a to head
        assert_eq!(
            s.recent_graphs,
            vec![PathBuf::from("/a"), PathBuf::from("/b")]
        );
    }

    #[test]
    fn push_recent_caps_to_recents_cap() {
        let mut s = GlobalAppState::default();
        for i in 0..(RECENTS_CAP + 5) {
            s.push_recent(PathBuf::from(format!("/g{i}")));
        }
        assert_eq!(s.recent_graphs.len(), RECENTS_CAP);
        // Most-recent-first.
        assert_eq!(s.recent_graphs[0], PathBuf::from(format!("/g{}", RECENTS_CAP + 4)));
    }

    #[test]
    fn default_is_empty() {
        let s = GlobalAppState::default();
        assert!(s.last_opened_graph.is_none());
        assert!(s.recent_graphs.is_empty());
        assert!(s.right_sidebar_visible.is_none());
    }

    #[test]
    fn new_applies_cap_and_dedup() {
        let paths: Vec<PathBuf> = (0..20).map(|i| PathBuf::from(format!("/g{i}"))).collect();
        let s = GlobalAppState::new(None, paths, None);
        assert_eq!(s.recent_graphs.len(), RECENTS_CAP);
        // /g19 is the most recent → head
        assert_eq!(s.recent_graphs[0], PathBuf::from("/g19"));
    }
}
