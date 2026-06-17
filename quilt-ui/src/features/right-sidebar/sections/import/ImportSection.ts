// ─── sections/import/ImportSection — GS-9 migration section registration ────
//
// Registers the MigrationPanel as a right-sidebar section.
// Priority 400 = utility range (below structural/content/block/cognitive).

import { memo } from 'react'
import { MigrationPanel } from '@features/import'
import type { RightSidebarSection } from '../types'

export const MIGRATION_SECTION_ID = 'migration'

const MigrationSectionComponent = memo(function MigrationSectionComponent() {
  return <MigrationPanel />
})

export const migrationSection: RightSidebarSection = {
  id: MIGRATION_SECTION_ID,
  label: 'Import',
  priority: 400,
  visible: true,
  component: MigrationSectionComponent,
}
