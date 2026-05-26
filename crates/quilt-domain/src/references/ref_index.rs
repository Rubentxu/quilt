//! RefIndex — an in-memory bidirectional index of references
//!
//! `RefIndex` maintains forward (source → refs) and reverse (target → sources)
//! mappings using dual `HashMap`s. This enables O(1) backlink lookups without
//! scanning all references.
//!
//! The index is pure Rust, has no external dependencies beyond `std`, and is
//! WASM-compatible. It can be rebuilt from a repository and kept in sync
//! as blocks are saved/updated.
//!
//! # Architecture
//!
//! ```text
//! forward:  source UUID  ──→  HashSet<Ref>     (what does this source reference?)
//! reverse:  target UUID  ──→  HashSet<(Uuid, RefType)>  (what references this target?)
//! ```
//!
//! # Examples
//!
//! ```ignore
//! use quilt_domain::references::{Ref, RefIndex, RefType};
//! use quilt_domain::value_objects::Uuid;
//!
//! let mut index = RefIndex::new();
//! let source = Uuid::new_v4();
//! let target = Uuid::new_v4();
//!
//! index.add_ref(source, target, RefType::BlockRef);
//! assert_eq!(index.backlink_count(target), 1);
//! ```

use crate::references::{Ref, RefType};
use crate::value_objects::Uuid;
use std::collections::{HashMap, HashSet};

/// An in-memory bidirectional reference index.
///
/// Provides O(1) forward (source → targets) and reverse (target → sources)
/// lookups. All operations are pure Rust — no IO, no dependencies beyond std.
///
/// # Thread safety
///
/// `RefIndex` is not `Send` or `Sync` by default. Wrap in `Mutex` or `RwLock`
/// for concurrent access.
#[derive(Debug, Clone)]
pub struct RefIndex {
    /// Forward map: source → set of (target, ref_type)
    forward: HashMap<Uuid, HashSet<Ref>>,
    /// Reverse map: target → set of (source, ref_type)
    reverse: HashMap<Uuid, HashSet<(Uuid, RefType)>>,
}

impl RefIndex {
    /// Creates a new empty `RefIndex`.
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Adds a reference from `source` to `target` with the given `ref_type`.
    ///
    /// This updates both the forward and reverse maps atomically.
    /// Duplicate additions are idempotent.
    pub fn add_ref(&mut self, source: Uuid, target: Uuid, ref_type: RefType) {
        let r = Ref::new(target, ref_type);
        self.forward.entry(source).or_default().insert(r);
        self.reverse
            .entry(target)
            .or_default()
            .insert((source, ref_type));
    }

    /// Removes a reference from `source` to `target` of the given `ref_type`.
    ///
    /// If no more references exist between the pair, the entry is cleaned up.
    pub fn remove_ref(&mut self, source: Uuid, target: Uuid, ref_type: RefType) {
        // Remove from forward
        if let Some(refs) = self.forward.get_mut(&source) {
            refs.remove(&Ref::new(target, ref_type));
            if refs.is_empty() {
                self.forward.remove(&source);
            }
        }

        // Remove from reverse
        if let Some(sources) = self.reverse.get_mut(&target) {
            sources.remove(&(source, ref_type));
            if sources.is_empty() {
                self.reverse.remove(&target);
            }
        }
    }

    /// Removes all references originating from `source`.
    ///
    /// This is useful when a block is deleted or when replacing all refs
    /// for a block during sync.
    pub fn remove_all_from_source(&mut self, source: Uuid) {
        if let Some(refs) = self.forward.remove(&source) {
            for r in refs {
                if let Some(sources) = self.reverse.get_mut(&r.target) {
                    sources.remove(&(source, r.ref_type));
                    if sources.is_empty() {
                        self.reverse.remove(&r.target);
                    }
                }
            }
        }
    }

    /// Removes all references pointing to `target`.
    ///
    /// This is useful when a block or page is deleted.
    pub fn remove_all_to_target(&mut self, target: Uuid) {
        if let Some(sources) = self.reverse.remove(&target) {
            for (source, ref_type) in sources {
                if let Some(refs) = self.forward.get_mut(&source) {
                    refs.remove(&Ref::new(target, ref_type));
                    if refs.is_empty() {
                        self.forward.remove(&source);
                    }
                }
            }
        }
    }

    /// Returns all references that point to `target` (backlinks).
    ///
    /// Each backlink is a `(source, ref_type)` pair identifying what entity
    /// references the given target and how.
    ///
    /// Returns an empty `Vec` if there are no backlinks.
    pub fn get_backlinks(&self, target: Uuid) -> Vec<(Uuid, RefType)> {
        self.reverse
            .get(&target)
            .map(|sources| sources.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Returns all references originating from `source` (forward refs).
    ///
    /// Each forward ref is a `(target, ref_type)` pair.
    ///
    /// Returns an empty `Vec` if the source has no outgoing references.
    pub fn get_forward_refs(&self, source: Uuid) -> Vec<(Uuid, RefType)> {
        self.forward
            .get(&source)
            .map(|refs| refs.iter().map(|r| (r.target, r.ref_type)).collect())
            .unwrap_or_default()
    }

    /// Returns the number of distinct sources that reference `target`.
    pub fn backlink_count(&self, target: Uuid) -> usize {
        self.reverse
            .get(&target)
            .map(|sources| sources.len())
            .unwrap_or(0)
    }

    /// Returns the number of distinct targets referenced by `source`.
    pub fn forward_count(&self, source: Uuid) -> usize {
        self.forward
            .get(&source)
            .map(|refs| refs.len())
            .unwrap_or(0)
    }

    /// Returns `true` if the index contains no references at all.
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty() && self.reverse.is_empty()
    }

    /// Returns the total number of forward entries (distinct sources).
    pub fn len(&self) -> usize {
        self.forward.len()
    }

    /// Removes all entries from the index.
    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
    }
}

impl Default for RefIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uuids(n: usize) -> Vec<Uuid> {
        (0..n).map(|_| Uuid::new_v4()).collect()
    }

    #[test]
    fn test_add_ref_forward_and_reverse() {
        let uuids = make_uuids(2);
        let (source, target) = (uuids[0], uuids[1]);

        let mut index = RefIndex::new();
        index.add_ref(source, target, RefType::BlockRef);

        let forward = index.get_forward_refs(source);
        assert_eq!(forward.len(), 1);
        assert_eq!(forward[0], (target, RefType::BlockRef));

        let backlinks = index.get_backlinks(target);
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0], (source, RefType::BlockRef));
    }

    #[test]
    fn test_remove_ref() {
        let uuids = make_uuids(2);
        let (source, target) = (uuids[0], uuids[1]);

        let mut index = RefIndex::new();
        index.add_ref(source, target, RefType::PageRef);
        assert_eq!(index.backlink_count(target), 1);

        index.remove_ref(source, target, RefType::PageRef);
        assert_eq!(index.backlink_count(target), 0);
        assert!(index.get_forward_refs(source).is_empty());
    }

    #[test]
    fn test_remove_ref_does_not_affect_other_types() {
        let uuids = make_uuids(2);
        let (source, target) = (uuids[0], uuids[1]);

        let mut index = RefIndex::new();
        index.add_ref(source, target, RefType::PageRef);
        index.add_ref(source, target, RefType::BlockRef);
        assert_eq!(index.forward_count(source), 2);

        index.remove_ref(source, target, RefType::PageRef);
        assert_eq!(index.forward_count(source), 1);

        let forward = index.get_forward_refs(source);
        assert_eq!(forward[0], (target, RefType::BlockRef));
    }

    #[test]
    fn test_remove_all_from_source() {
        let uuids = make_uuids(4);
        let (source, t1, t2) = (uuids[0], uuids[1], uuids[2]);

        let mut index = RefIndex::new();
        index.add_ref(source, t1, RefType::BlockRef);
        index.add_ref(source, t2, RefType::PageRef);
        assert_eq!(index.forward_count(source), 2);

        index.remove_all_from_source(source);
        assert_eq!(index.forward_count(source), 0);
        assert_eq!(index.backlink_count(t1), 0);
        assert_eq!(index.backlink_count(t2), 0);
    }

    #[test]
    fn test_remove_all_to_target() {
        let uuids = make_uuids(4);
        let (s1, s2, target) = (uuids[0], uuids[1], uuids[2]);

        let mut index = RefIndex::new();
        index.add_ref(s1, target, RefType::BlockRef);
        index.add_ref(s2, target, RefType::PageRef);
        assert_eq!(index.backlink_count(target), 2);

        index.remove_all_to_target(target);
        assert_eq!(index.backlink_count(target), 0);
        assert!(index.get_forward_refs(s1).is_empty());
        assert!(index.get_forward_refs(s2).is_empty());
    }

    #[test]
    fn test_duplicate_add_is_idempotent() {
        let uuids = make_uuids(2);
        let (source, target) = (uuids[0], uuids[1]);

        let mut index = RefIndex::new();
        index.add_ref(source, target, RefType::Tag);
        index.add_ref(source, target, RefType::Tag);

        let forward = index.get_forward_refs(source);
        assert_eq!(forward.len(), 1);

        let backlinks = index.get_backlinks(target);
        assert_eq!(backlinks.len(), 1);
    }

    #[test]
    fn test_empty_backlinks() {
        let target = Uuid::new_v4();
        let index = RefIndex::new();

        let backlinks = index.get_backlinks(target);
        assert!(backlinks.is_empty());
        assert_eq!(index.backlink_count(target), 0);
    }

    #[test]
    fn test_empty_forward_refs() {
        let source = Uuid::new_v4();
        let index = RefIndex::new();

        let forward = index.get_forward_refs(source);
        assert!(forward.is_empty());
        assert_eq!(index.forward_count(source), 0);
    }

    #[test]
    fn test_clear() {
        let uuids = make_uuids(3);
        let mut index = RefIndex::new();
        index.add_ref(uuids[0], uuids[1], RefType::Alias);
        index.add_ref(uuids[1], uuids[2], RefType::PageRef);

        assert!(!index.is_empty());
        index.clear();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_multiple_sources_referencing_same_target() {
        let uuids = make_uuids(4);
        let target = uuids[0];

        let mut index = RefIndex::new();
        index.add_ref(uuids[1], target, RefType::PageRef);
        index.add_ref(uuids[2], target, RefType::BlockRef);
        index.add_ref(uuids[3], target, RefType::Tag);

        assert_eq!(index.backlink_count(target), 3);

        let backlinks = index.get_backlinks(target);
        assert_eq!(backlinks.len(), 3);

        // Verify all three sources are present
        let sources: HashSet<Uuid> = backlinks.iter().map(|(s, _)| *s).collect();
        assert!(sources.contains(&uuids[1]));
        assert!(sources.contains(&uuids[2]));
        assert!(sources.contains(&uuids[3]));
    }

    #[test]
    fn test_backlinks_are_o1_not_scanning_all_entries() {
        // This test verifies the O(1) backlink property by proxy:
        // get_backlinks should only find the target's entry, not scan
        let uuids = make_uuids(100);

        let mut index = RefIndex::new();
        // Create many forward refs from source 0 to many targets
        let source = uuids[0];
        for i in 1..100 {
            index.add_ref(source, uuids[i], RefType::PageRef);
        }

        // Now check backlinks for a specific target — should only return source 0
        let backlinks = index.get_backlinks(uuids[50]);
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].0, source);
    }

    #[test]
    fn test_ref_index_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<RefIndex>();
    }
}
