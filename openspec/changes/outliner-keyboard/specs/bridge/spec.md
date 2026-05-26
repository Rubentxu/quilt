# Bridge API

## Purpose

HTTP client for server communication. Provides block CRUD operations and new move/delete operations for keyboard handling.

## Requirements

### Requirement: Existing Block CRUD

The system SHALL continue providing get_page_blocks, create_block, and update_block operations.

#### Scenario: Fetch page blocks

- GIVEN page_id "page-1"
- WHEN get_page_blocks("page-1") is called
- THEN returns Vec<BlockDto> for all blocks on page
- AND blocks are ordered by order field

#### Scenario: Create block

- GIVEN page_id, parent_id, content, and order
- WHEN create_block(request) is called
- THEN new block is created on server
- AND BlockDto is returned

#### Scenario: Update block content

- GIVEN block_id and new content
- WHEN update_block(block_id, content) is called
- THEN block content is updated on server

### Requirement: Delete Block API

The system SHALL provide delete_block operation for removing blocks.

#### Scenario: Delete empty block

- GIVEN block_id of empty block with no children
- WHEN delete_block(block_id) is called
- THEN block is removed from server
- AND returns success

#### Scenario: Delete block with children fails

- GIVEN block_id of block with children
- WHEN delete_block(block_id) is called
- THEN returns BlockHasChildren error
- AND no deletion occurs

### Requirement: Move Block API

The system SHALL provide move_block for indent, outdent, and reorder operations.

#### Scenario: Move block to new parent (indent)

- GIVEN block_id, new_parent_id, and new_order
- WHEN move_block(block_id, new_parent_id, new_order) is called
- THEN block's parent_id is updated
- AND block's order is updated
- AND block's level is set to parent's level + 1

#### Scenario: Move block to root (outdent)

- GIVEN block_id and new_parent_id (None for root)
- WHEN move_block(block_id, None, new_order) is called
- THEN block's parent_id becomes None
- AND block's level becomes 1

### Requirement: Batch Update Support

The system MAY provide batch update for syncing multiple block changes.

#### Scenario: Batch sync after multiple operations

- GIVEN [delete, indent, split] operations queued
- WHEN sync_blocks(operations) is called
- THEN all operations are applied atomically
- OR partial results are returned with error

### Requirement: Optimistic Rollback

The system SHALL support rollback on server error.

#### Scenario: Server error on delete

- GIVEN block was optimistically deleted from local state
- WHEN delete_block API returns error
- THEN local state is restored
- AND user is notified via toast

### Requirement: Error Handling

The system SHALL return typed errors for client handling.

#### Scenario: Block not found

- GIVEN invalid block_id
- WHEN any block operation is called
- THEN BlockNotFound error is returned

#### Scenario: Concurrent modification

- GIVEN block was modified by another client
- WHEN update_block is called
- THEN ConcurrentEdit error is returned
- AND client resolves conflict
