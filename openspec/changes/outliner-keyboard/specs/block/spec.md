# Block Component

## Purpose

Block display component that owns editing state and passes callbacks to BlockEditor. Coordinates between page-level state and individual block editing.

## Requirements

### Requirement: Editing State Management

The system SHALL toggle between display and edit mode using an editing signal owned by the block component.

#### Scenario: Click toggles edit mode

- GIVEN a block is in display mode
- WHEN the block content area is clicked
- THEN editing signal is set to true
- AND BlockEditor is rendered

#### Scenario: Escape exits edit mode

- GIVEN a block is in edit mode
- WHEN on_cancel callback is invoked
- THEN editing signal is set to false
- AND display mode resumes

### Requirement: Callback Passage to BlockEditor

The system SHALL pass all keyboard operation callbacks to BlockEditor.

#### Scenario: All callbacks passed

- GIVEN a block in edit mode
- WHEN BlockEditor is rendered
- THEN it receives: on_enter, on_tab, on_shift_tab, on_backspace, on_split, on_merge, on_cancel callbacks

### Requirement: Page-Level State Access

The system SHALL access page-level RwSignal<Vec<BlockDto>> for block mutations.

#### Scenario: Reads blocks from page signal

- GIVEN page signal contains blocks [A, B, C]
- WHEN block needs to find sibling
- THEN it reads from page signal to find A's previous sibling

#### Scenario: Updates propagate to page

- GIVEN page signal contains [A, B]
- WHEN B is indented to become child of A
- THEN page signal updates to reflect new hierarchy

### Requirement: Focus Management

The system SHALL coordinate focus transfer with parent block component.

#### Scenario: Requests focus on new block

- GIVEN a new block was created by Enter key
- WHEN the new block renders
- THEN focus is set to the new block's editor

#### Scenario: Focus moves to sibling on delete

- GIVEN block B (previous sibling of C) exists
- WHEN C is deleted
- THEN focus moves to B

### Requirement: Children Collapse State

The system SHALL respect collapsed state when deleting blocks.

#### Scenario: Cannot delete block with collapsed children

- GIVEN block A has children and is collapsed
- WHEN Backspace is pressed on empty A
- THEN A is NOT deleted
- AND error toast is shown
