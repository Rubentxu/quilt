// ─── sections/block-properties/BlockPropertiesSection ──────────────────────
//
// BlockPropertiesPanel wired as a block-scoped section.
// Priority: 200 (block-scoped range 200-299)
// Predicate: only when a block is selected
//
// ## ADR-0020 reconciliation (ADR-0031)
// ADR-0020's "block-focus-populates-header" behaviour is SUPERSEDED.
// Block-level property editing lives in the right sidebar, not the header.
// Page-level properties remain in the header (2-state CRDT invariant preserved).

import { memo } from 'react'
import { BlockPropertiesPanel } from '@features/properties/BlockPropertiesPanel'
import type { RightSidebarSection } from '../types'
import type { Selection, BlockSelection } from '../../selection/types'
import { isBlockSelection } from '../../selection/types'

interface BlockPropertiesSectionProps {
  selection: Selection
}

const BlockPropertiesSectionComponent = memo(function BlockPropertiesSectionComponent({ selection }: BlockPropertiesSectionProps) {
  if (!isBlockSelection(selection)) return null

  return (
    <BlockPropertiesPanel
      blockId={selection.blockId}
      onClose={() => {}}
    />
  )
})

export const BLOCK_PROPERTIES_SECTION_ID = 'block-properties'

export const blockPropertiesSection: RightSidebarSection = {
  id: BLOCK_PROPERTIES_SECTION_ID,
  label: 'Properties',
  priority: 200,
  visible: true,
  // Only show when a block is selected
  predicate: (selection): boolean => isBlockSelection(selection),
  component: BlockPropertiesSectionComponent,
}
