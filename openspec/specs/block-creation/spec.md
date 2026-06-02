# Delta for Block Creation

## MODIFIED Requirements

### Requirement: Create Block with Position

The system SHALL accept a `precedingBlockId` parameter in the create block request. When provided, the new block SHALL be inserted immediately after the specified preceding block, with its order value calculated as the midpoint between the preceding block's order and the next sibling's order.

When `precedingBlockId` is `null` or absent, the system SHALL append the block at the end of the sibling list (current behavior).

(Previously: blocks were always appended at `order: 1.0` regardless of existing sibling order.)

#### Scenario: Insert after specific block

- GIVEN a page with blocks ordered as `[A(order=1.0), B(order=2.0)]`
- WHEN `POST /api/blocks` with `{content: "C", pageName: "test", precedingBlockId: "A"}`
- THEN block C SHALL be created with `order` between 1.0 and 2.0 (e.g., 1.5)

#### Scenario: Insert at end (no preceding)

- GIVEN a page with blocks ordered as `[A(order=1.0), B(order=2.0)]`
- WHEN `POST /api/blocks` with `{content: "C", pageName: "test"}` (no precedingBlockId)
- THEN block C SHALL be created with `order` greater than all existing siblings

#### Scenario: Insert after last block

- GIVEN a page with blocks ordered as `[A(order=1.0), B(order=2.0)]`
- WHEN `POST /api/blocks` with `{content: "C", pageName: "test", precedingBlockId: "B"}`
- THEN block C SHALL be created with `order` greater than 2.0

#### Scenario: Insert with parent

- GIVEN a block A with children `[C1(order=1.0), C2(order=2.0)]`
- WHEN `POST /api/blocks` with `{content: "C3", pageName: "test", parentId: "A", precedingBlockId: "C1"}`
- THEN block C3 SHALL be created as child of A with order between C1 and C2

#### Scenario: Preceding block not found

- GIVEN a `precedingBlockId` that does not exist
- WHEN the request is processed
- THEN the system SHALL return a 400 error with message "Preceding block not found"

#### Scenario: Fractional order gap exhausted

- GIVEN two adjacent blocks with orders 1.0 and 1.0000001
- WHEN a new block is inserted between them
- THEN the system SHALL re-index sibling orders to create space, then insert

## ADDED Requirements

### Requirement: Fractional Order Calculation

The system SHALL provide a utility that calculates the midpoint order between two f64 values. If the gap between the two values is less than a defined epsilon (0.00001), the system SHALL re-index all siblings with evenly spaced orders before calculating the midpoint.

#### Scenario: Normal midpoint

- GIVEN preceding order = 1.0, next order = 2.0
- THEN calculated order SHALL be 1.5

#### Scenario: Gap too small triggers re-index

- GIVEN preceding order = 1.0, next order = 1.000001
- AND there are 5 siblings total
- THEN siblings SHALL be re-indexed (e.g., 1.0, 2.0, 3.0, 4.0, 5.0)
- AND new block SHALL be inserted at midpoint between the two target siblings

### Requirement: Frontend Sends Preceding Block ID

The frontend `bridge::create_block` function SHALL accept an optional `preceding_block_id` parameter and include it as `precedingBlockId` in the request body.

#### Scenario: Split block sends preceding ID

- GIVEN user splits block A at cursor position
- WHEN the frontend creates the new block B
- THEN the request SHALL include `precedingBlockId: "A"` so B appears after A
