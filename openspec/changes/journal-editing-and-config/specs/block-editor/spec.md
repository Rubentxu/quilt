# Delta: Block Editor — Autosave Requirement

## MODIFIED Requirements

### Requirement: Content Editable Container

(Previously: contenteditable div displays block content)

The system SHALL render a contenteditable div that displays and edits block content AND persist changes to the server on blur.

#### Scenario: Displays block content

- GIVEN a block with content "Hello world"
- WHEN the block editor renders
- THEN the contenteditable contains "Hello world"

#### Scenario: Preserves cursor position on update

- GIVEN cursor is at offset 5 in content
- WHEN content signal updates with same text
- THEN cursor position is preserved at offset 5

#### Scenario: Saves to server on blur

- GIVEN a block with id "b-1" and content "Hello world"
- WHEN the user edits to "Hello world!" and blurs
- THEN bridge::update_block("b-1", "Hello world!") is called
- AND block content is persisted to server

### Requirement: Optimistic Update Coordination

(Previously: signal updated before server call)

The system SHALL coordinate with page-level RwSignal for optimistic updates AND call the bridge API for server persistence.

#### Scenario: Signals updated before server call

- GIVEN page-level blocks signal
- WHEN split operation occurs
- THEN signal is updated immediately
- AND async server call fires in background

#### Scenario: Escape reverts content

- GIVEN block has original content "Saved"
- WHEN Escape is pressed
- THEN content reverts to "Saved"
- AND on_cancel is called
- AND update_block is NOT called (content unchanged)
