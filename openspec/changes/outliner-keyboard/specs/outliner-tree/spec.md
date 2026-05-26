# Outliner Tree Operations

## Purpose

Tree structure operations on flat block lists. Provides indent, outdent, split, and merge operations on BlockDto collections.

## Requirements

### Requirement: Tree Building

The system SHALL build hierarchical tree from flat Vec<BlockDto> using parent_id and order.

#### Scenario: Build tree from flat list

- GIVEN flat list [root1, child1a, child1b, root2]
- WHEN build_tree(blocks) is called
- THEN returns tree with root1 at top containing children child1a, child1b
- AND root2 as separate top-level node

### Requirement: Flatten Tree

The system SHALL convert tree back to flat list preserving order.

#### Scenario: Flatten to list

- GIVEN tree with root1 containing [child1a, child1b]
- WHEN flatten_tree(tree) is called
- THEN returns flat [root1, child1a, child1b]

### Requirement: Indent Operation

The system SHALL indent a block to become last child of its previous sibling.

#### Scenario: Indent block with previous sibling

- GIVEN block B with previous sibling A at same level
- WHEN indent(blocks, block_id: B) is called
- THEN B's parent_id becomes A's id
- AND B's order becomes after A's last child
- AND B's level becomes A's level + 1

#### Scenario: Indent first child fails

- GIVEN block A is first child (no previous sibling)
- WHEN indent(blocks, block_id: A) is called
- THEN returns IndentError::NoPreviousSibling

### Requirement: Outdent Operation

The system SHALL outdent a block to become sibling of its parent.

#### Scenario: Outdent block with parent

- GIVEN block C with parent B at level 2
- WHEN outdent(blocks, block_id: C) is called
- THEN C's parent_id becomes B's parent_id
- AND C's order becomes after B
- AND C's level becomes B's level

#### Scenario: Outdent root block fails

- GIVEN block A with no parent (root level)
- WHEN outdent(blocks, block_id: A) is called
- THEN returns OutdentError::NoParent

### Requirement: Split Block Operation

The system SHALL split a block at given cursor position into two blocks.

#### Scenario: Split at middle position

- GIVEN block with content "Hello world" at cursor position 5
- WHEN split_block(blocks, block_id, cursor: 5) is called
- THEN first block retains "Hello"
- AND second block is created with " world"
- AND second block has same parent and correct order

#### Scenario: Split at end creates empty block

- GIVEN block with content "Hello" at cursor position 5
- WHEN split_block(blocks, block_id, cursor: 5) is called
- THEN first block retains "Hello"
- AND second block is created with empty content

### Requirement: Merge Blocks Operation

The system SHALL merge source block content into target block and delete source.

#### Scenario: Merge with previous sibling

- GIVEN block B with previous sibling A
- WHEN merge_blocks(blocks, target: B, source: A) is called
- THEN A's content is appended to B's content
- AND A is removed from blocks
- AND B's order is unchanged

#### Scenario: Merge at end of page

- GIVEN last block with no next sibling
- WHEN merge with next is attempted
- THEN returns MergeError::NoNextSibling

### Requirement: Order Calculation

The system SHALL use fractional indexing for correct sibling ordering.

#### Scenario: Calculate order between siblings

- GIVEN siblings A (order: 1.0) and B (order: 2.0)
- WHEN calculate_order(A, B) is called for new block between them
- THEN returns 1.5

#### Scenario: Calculate order at end

- GIVEN last sibling with order 2.0
- WHEN calculate_order(last, None) is called
- THEN returns 3.0
