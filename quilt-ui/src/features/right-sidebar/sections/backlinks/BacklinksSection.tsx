// ─── sections/backlinks/BacklinksSection ───────────────────────────────────
//
// BacklinksPanel wrapped as a RightSidebarSection.
// Priority: 50 (structural, same as former BacklinksPanel placement)
// Predicate: always visible (page-level fallback always applies)
//
// Migration: BacklinksPanel was previously rendered as a standalone column
// in AppShell. It is now a registered section in the right sidebar.

import { memo } from 'react'
import { BacklinksPanel } from '@features/references/BacklinksPanel'
import type { RightSidebarSection, SectionPriority } from '../types'
import type { Selection } from '../../selection/types'

interface BacklinksSectionProps {
  selection: Selection
}

const BacklinksSectionComponent = memo(function BacklinksSectionComponent({ selection }: BacklinksSectionProps) {
  const pageName =
    selection?.type === 'block'
      ? selection.pageName
      : selection?.type === 'page'
        ? selection.pageName
        : null

  return <BacklinksPanel pageName={pageName} isOpen={true} defaultExpanded={true} />
})

export const BACKLINKS_SECTION_ID = 'backlinks'

export const backlinksSection: RightSidebarSection = {
  id: BACKLINKS_SECTION_ID,
  label: 'Backlinks',
  priority: 50,
  visible: true,
  // No predicate — BacklinksPanel handles its own empty state
  component: BacklinksSectionComponent,
}

export const BACKLINKS_PRIORITY: SectionPriority = 50
