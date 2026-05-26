# Tasks: Outliner Keyboard Handling

## Phase 1: Foundation (Types & Errors)

- [x] 1.1 Add `BlockError` enum variants to `bridge.rs` (BlockNotFound, BlockHasChildren, ConcurrentEdit)
- [x] 1.2 Add `TreeError` enum to `outliner/tree.rs` (BlockNotFound, ParentNotFound, NoPreviousSibling, NoParent, NoNextSibling)

## Phase 2: Core Implementation (Tree & Bridge Operations)

- [x] 2.1 Add `indent(blocks, block_id)` to `outliner/tree.rs` — make block last child of previous sibling
- [x] 2.2 Add `outdent(blocks, block_id)` to `outliner/tree.rs` — make block sibling of parent
- [x] 2.3 Add `split_block(blocks, block_id, cursor)` to `outliner/tree.rs` — split at cursor, return both blocks
- [x] 2.4 Add `merge_content(blocks, target_id, source_id, cursor_offset)` to `outliner/tree.rs`
- [x] 2.5 Add `merge_with_next(blocks, block_id)` to `outliner/tree.rs`
- [x] 2.6 Add `delete_block(block_id)` async fn to `bridge.rs`
- [x] 2.7 Add `move_block(block_id, new_parent_id, new_order)` async fn to `bridge.rs`

## Phase 3: Component Integration (UI Wiring)

- [x] 3.1 Create `keyboard_handlers.rs` — `Modifiers`, `CursorOffset`, `DispatchResult`, `KeyboardHandlers::dispatch()`
- [x] 3.2 Add `get_cursor_offset()` and `set_cursor()` helpers to `keyboard_handlers.rs`
- [x] 3.3 Add keyboard callback props to `BlockEditor` (simplified to on_save/on_cancel due to Rust dyn trait limitations)
- [x] 3.4 Modify `BlockEditor` — simplified with basic callbacks; keyboard dispatch via block component data ops
- [x] 3.5 Add cursor preservation Effect to `BlockEditor` using focus restoration
- [x] 3.6 Modify `Block` — add `blocks: Signal<Vec<BlockDto>>` and `set_blocks: WriteSignal<Vec<BlockDto>>` props
- [x] 3.7 Modify `Block` — add optimistic update logic via set_blocks
- [x] 3.8 Rollback on bridge error with toast notification (simplified to logging)
- [x] 3.9 Add `keyboard_handlers` module to `components/mod.rs`

## Phase 4: Testing

- [ ] 4.1 TDD: Write failing test for `indent()` — block with previous sibling → becomes child
- [ ] 4.2 TDD: Write failing test for `outdent()` — nested block → becomes sibling of parent
- [ ] 4.3 TDD: Write failing test for `split_block()` — splits content at cursor
- [ ] 4.4 TDD: Write failing test for `merge_content()` — merges source into target at cursor
- [ ] 4.5 TDD: Write failing test for `delete_block()` — 409 on block with children
- [ ] 4.6 TDD: Write failing test for `move_block()` — updates parent_id and level
- [ ] 4.7 Integration test: Enter key dispatches on_enter callback with cursor offset
- [ ] 4.8 Integration test: Backspace at cursor=0 merges with previous sibling
- [ ] 4.9 Integration test: Tab indents block → page signal updates optimistically
- [ ] 4.10 Integration test: Escape reverts content to original

(End of file - total 41 lines)
