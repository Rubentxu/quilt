# Block Editor Component

## Purpose

Contenteditable text editing component for blocks. Handles cursor-aware text splitting and integrates with keyboard handlers via page-level callbacks.

## Requirements

### Requirement: Content Editable Container

The system SHALL render a contenteditable div that displays and edits block content.

#### Scenario: Displays block content

- GIVEN a block with content "Hello world"
- WHEN the block editor renders
- THEN the contenteditable contains "Hello world"

#### Scenario: Preserves cursor position on update

- GIVEN cursor is at offset 5 in content
- WHEN content signal updates with same text
- THEN cursor position is preserved at offset 5

### Requirement: Cursor-Aware Split

The system SHALL split block content at cursor position when requested by keyboard handler.

#### Scenario: Split at cursor middle

- GIVEN block content is "Hello world" with cursor between "ello" and " world"
- WHEN split_at(cursor_offset: 5) is called
- THEN first block receives "Hello"
- AND second block receives " world"

#### Scenario: Split at cursor end

- GIVEN block content is "Hello" with cursor at end (offset 5)
- WHEN split_at(cursor_offset: 5) is called
- THEN first block receives "Hello"
- AND second block receives empty string

### Requirement: Content Merge

The system SHALL merge content from another block at cursor position.

#### Scenario: Merge content at cursor

- GIVEN current block has "Hello" with cursor at end
- WHEN merge_content(" world") is called
- THEN content becomes "Hello world"

### Requirement: Page-Level State Callbacks

The system SHALL receive callbacks from parent block for all mutation operations.

#### Scenario: Enter callback triggers split

- GIVEN block editor has on_enter callback
- WHEN Enter key is pressed
- THEN on_enter is invoked with cursor position

#### Scenario: Tab callback triggers indent

- GIVEN block editor has on_tab callback
- WHEN Tab key is pressed
- THEN on_tab is invoked

#### Scenario: Backspace callback at start

- GIVEN block editor has on_backspace callback
- WHEN Backspace is pressed at start of block
- THEN on_backspace is invoked with cursor position 0

#### Scenario: Escape reverts content

- GIVEN block has original content "Saved"
- WHEN Escape is pressed
- THEN content reverts to "Saved"
- AND on_cancel is called

### Requirement: DOM Cursor Integration

The system SHALL use window.getSelection() for cursor position and node_ref for focus management.

#### Scenario: Get cursor offset

- GIVEN editor has focus with cursor between characters
- WHEN get_cursor_offset() is called
- THEN returns character offset of cursor

#### Scenario: Set cursor position

- GIVEN editor has content "Hello"
- WHEN set_cursor(offset: 3) is called
- THEN cursor is placed between "Hel" and "lo"

### Requirement: Optimistic Update Coordination

The system SHALL coordinate with page-level RwSignal for optimistic updates.

#### Scenario: Signals updated before server call

- GIVEN page-level blocks signal
- WHEN split operation occurs
- THEN signal is updated immediately
- AND async server call fires in background
