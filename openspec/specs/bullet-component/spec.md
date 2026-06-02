# Bullet Component Specification

## Purpose

Render Quilt-style visual bullets for blocks with proper React DOM structure, separating the visual bullet/marker from the drag handle.

> **Note**: With the migration from Leptos to React + TipTap, the DOM structure differs from Logseq's original `.bullet-container > .bullet-link-wrap > .bullet`. The current implementation uses a flat React component hierarchy with `data-testid` selectors.

## Requirements

### Requirement: Block Row Structure

The system SHALL render each block as a `<div class="block-row">` containing a bullet/marker button, optional badges (marker, priority), content area, and action buttons.

#### Scenario: Block row renders with data-testid

- GIVEN a block with id `abc-123`
- WHEN the block renders
- THEN a `<div>` SHALL exist with `data-testid="block-row-abc-123"` and class `block-row`

### Requirement: Bullet Visualization

The bullet SHALL visually indicate whether the block has children (chevron for parent blocks, dot for leaves).

#### Scenario: Leaf block bullet

- GIVEN a block with no children
- WHEN the block renders in display mode
- THEN a `<button class="block-bullet">` SHALL render containing a small circular dot (`<div>` with 8px width/height and `border-radius: var(--radius-pill)`)

#### Scenario: Parent block bullet expanded

- GIVEN a block with children that is not collapsed
- THEN a `<button class="block-bullet">` SHALL render containing a `ChevronDown` icon (▼)

#### Scenario: Parent block bullet collapsed

- GIVEN a block with children that IS collapsed
- THEN a `<button class="block-bullet">` SHALL render containing a `ChevronRight` icon (▶)

### Requirement: Bullet Click Handler

Clicking the bullet SHALL toggle collapse for parent blocks. Clicking a leaf block's bullet SHALL NOT trigger collapse behavior.

#### Scenario: Click parent bullet toggles collapse

- GIVEN a block with children that is expanded
- WHEN the user clicks the `<button class="block-bullet">`
- THEN the block SHALL collapse and children SHALL be hidden

#### Scenario: Click leaf bullet is a no-op

- GIVEN a block with no children
- WHEN the user clicks the `<button class="block-bullet">`
- THEN nothing SHALL happen (no collapse toggle, no selection)

### Requirement: Drag Handle Separation

The drag handle SHALL be a separate element from the visual bullet. The bullet is for click interaction (collapse). The drag handle (grip icon) is for drag-and-drop.

#### Scenario: Drag starts from grip, not bullet

- GIVEN a block in display mode
- WHEN the user initiates a drag on `<div class="drag-handle">` (containing `GripVertical` icon)
- THEN the drag SHALL start from the dedicated drag handle (via `@dnd-kit` sortable props)

#### Scenario: Click on bullet does not start drag

- GIVEN a block in display mode
- WHEN the user clicks the bullet button (mousedown + mouseup without movement)
- THEN a click event SHALL fire and NO drag SHALL start (bullet is `<button>`, drag handle is separate)

### Requirement: Test Selectors

Components SHALL use `data-testid` attributes for E2E test targeting instead of CSS class names.

#### Scenario: Block row has stable test ID

- GIVEN a rendered block with id `abc-123`
- THEN `[data-testid="block-row-abc-123"]` SHALL be present

#### Scenario: Bullet has accessible label

- GIVEN a parent block that is collapsed
- THEN the bullet button SHALL have `aria-label="Expand block"`
- GIVEN a parent block that is expanded
- THEN the bullet button SHALL have `aria-label="Collapse block"`
- GIVEN a leaf block
- THEN the bullet button SHALL have `aria-label="Bullet"`

### Requirement: Indentation with Stripe Lines

Blocks SHALL render with visual indentation lines (stripes) indicating nesting depth.

#### Scenario: Indented block shows stripes

- GIVEN a block at indent level 2
- WHEN the block renders
- THEN vertical stripe lines SHALL appear at each indentation level, positioned at `left: {level * 24 + 11}px`
