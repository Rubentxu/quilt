# Keyboard Handlers Component

## Purpose

Centralized key event dispatch for outliner block editing. Maps keyboard events to operations while managing cursor state and IME composition.

## Requirements

### Requirement: Key Event Dispatch

The system SHALL dispatch keydown events to appropriate handlers based on key name and modifier state. The handler MUST prevent browser default for intercepted keys.

#### Scenario: Enter key creates new block

- GIVEN a block is in editing mode with cursor at end of non-empty content
- WHEN Enter key is pressed without modifiers
- THEN the block content is split at cursor position
- AND a new block is created below with remaining content
- AND focus moves to the new block

#### Scenario: Shift+Enter inserts newline

- GIVEN a block is in editing mode
- WHEN Shift+Enter is pressed
- THEN browser default insertion of newline occurs
- AND no block operation is triggered

#### Scenario: Tab indents block

- GIVEN a block is in editing mode
- WHEN Tab key is pressed without modifiers
- THEN the current block is indented to become child of previous sibling
- AND focus remains in the block

#### Scenario: Shift+Tab outdents block

- GIVEN an indented block is in editing mode
- WHEN Shift+Tab is pressed
- THEN the block moves to parent's level as sibling of parent
- AND focus remains in the block

#### Scenario: Backspace merges or deletes

- GIVEN a block is in editing mode with cursor at start
- WHEN Backspace is pressed on non-empty block
- THEN content is merged with previous sibling
- AND current block is deleted
- AND focus moves to end of previous sibling

#### Scenario: Backspace on empty block deletes

- GIVEN an empty block is in editing mode
- WHEN Backspace is pressed
- THEN the block is deleted
- AND focus moves to previous sibling

#### Scenario: Escape cancels editing

- GIVEN a block is in editing mode with unsaved changes
- WHEN Escape is pressed
- THEN content is reverted to saved state
- AND editing mode is exited

#### Scenario: Ctrl+Enter explicit split

- GIVEN a block is in editing mode with cursor in middle of content
- WHEN Ctrl+Enter is pressed
- THEN block is split at cursor position
- AND new block receives remaining content
- AND focus moves to new block

#### Scenario: Ctrl+Backspace merges with next

- GIVEN a block is in editing mode
- WHEN Ctrl+Backspace is pressed
- THEN next sibling's content is appended to current block
- AND next sibling is deleted
- AND focus remains at merge point

### Requirement: IME Composition Handling

The system SHOULD ignore Enter key events during active IME composition.

#### Scenario: Enter during composition is ignored

- GIVEN a block is in editing mode with active IME composition
- WHEN Enter is pressed
- THEN no block split occurs
- AND composition continues

### Requirement: Cursor Position Preservation

The system SHALL preserve cursor position across re-renders using node_ref and explicit focus restoration.

#### Scenario: Cursor preserved after indent

- GIVEN a block has cursor at position 5
- WHEN Tab is pressed to indent the block
- THEN after re-render, cursor is at position 5 in the indented block

### Requirement: Focus Management

The system SHALL manage focus transfer between blocks during split, merge, indent, and outdent operations.

#### Scenario: Focus moves to new block on split

- GIVEN a block has focus at cursor position
- WHEN Enter is pressed to split
- THEN the new block receives focus

#### Scenario: Focus moves to previous sibling on merge

- GIVEN block B has focus
- WHEN Backspace merges B into previous sibling A
- THEN focus moves to end of A
