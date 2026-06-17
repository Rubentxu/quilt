// ─── sections/types — RightSidebarSection types ─────────────────────────────
//
// Immutable section descriptors. Sections are registered at startup via
// side-effect barrel imports. The registry is a LEAF module — it does
// NOT import any feature panels; only the barrel imports features.

import type { Selection } from '../selection/types'

/**
 * Scoping predicate: a section can be hidden when the selection doesn't
 * match certain criteria. When the predicate returns false, the section
 * is skipped during render even if it has content.
 *
 * Example: BlockPropertiesPanel only renders when a block is selected.
 *   predicate: (sel) => sel?.type === 'block'
 */
export type SectionPredicate = (selection: Selection) => boolean

/**
 * Priority determines render order within the sidebar. Lower numbers
 * render first. Ties are broken by registration order.
 *
 * ## Reserved priority ranges
 *   0-99   : Structural (Backlinks, Outline)
 *   100-199: Page-level content (Table of Contents)
 *   200-299: Block-scoped (BlockProperties)
 *   300-399: Cognitive panels
 *   400+   : Utility / Debug
 */
export type SectionPriority = number

/**
 * A single registered section in the RightSidebar.
 */
export interface RightSidebarSection {
  /**
   * Stable identifier unique within the sidebar. Used by tests and
   * for keyboard shortcut routing.
   */
  id: string

  /**
   * Human-readable label shown in the sidebar tab.
   */
  label: string

  /**
   * Controls render order. Lower = earlier. Ties broken by reg order.
   */
  priority: SectionPriority

  /**
   * When false, the section is hidden from the tab bar and never rendered.
   * Useful for feature-flagged sections.
   */
  visible: boolean

  /**
   * Optional predicate — when false the section is skipped even if visible.
   * Omitting the predicate means "always show when visible".
   */
  predicate?: SectionPredicate

  /**
   * The React component that renders the section content.
   * Receives `selection` as a prop so it can adjust rendering.
   */
  component: React.ComponentType<{ selection: Selection }>
}

/**
 * Result of the main-action ranker.
 */
export interface RankedAction {
  /** The section that produced this action. */
  sectionId: string
  /** Confidence score in [0, 1]. */
  confidence: number
  /** The suggested main action label. */
  label: string
  /** Optional onClick handler for the action. */
  onClick?: () => void
}
