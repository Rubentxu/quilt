# Block Autosave Specification

## Purpose

Block content edits MUST be persisted to the server when the user leaves edit mode (blur, navigation, or structural operation). This follows Logseq's save-block-on-blur pattern.

## Requirements

### Requirement: Block Content Saved On Blur

The system SHALL persist block content to the server when focus leaves the editor.

#### Scenario: Type and blur saves

- GIVEN user is editing block "B1" content
- WHEN user types "Hello world" and clicks outside the editor
- THEN bridge::update_block(B1.id, "Hello world") is called
- AND block content is saved to server

#### Scenario: Type and navigate away saves

- GIVEN user is editing block "B1"
- WHEN user presses ArrowDown to move to next block
- THEN the previous block's content is saved to server

### Requirement: Bridge Update Block Exists

The bridge SHALL provide update_block(block_id, content) that calls PUT /api/v1/blocks/:id.

#### Scenario: Update block via REST

- GIVEN block with id "block-123" and content "Updated text"
- WHEN bridge::update_block("block-123", "Updated text") is called
- THEN HTTP PUT /api/v1/blocks/block-123 is sent with {content: "Updated text"}
- AND server returns updated BlockDto

### Requirement: Optimistic Update With Async Save

The system SHALL update local state immediately and persist to server asynchronously.

#### Scenario: Local state updates before server

- GIVEN user edits block content from "Old" to "New"
- WHEN the change is saved
- THEN local signal is updated to "New" immediately
- AND server API call fires in background
- AND if server call fails, local state is NOT rolled back (fire-and-forget for MVP)

### Requirement: Empty Block Does Not Save

The system SHALL NOT call update_block when the content is empty.

#### Scenario: Empty content not persisted

- GIVEN block has content "Hello"
- WHEN user clears all content and blurs
- THEN update_block is NOT called
- AND block retains its previous content on next load
