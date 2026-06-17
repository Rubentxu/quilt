// ─── rankMainAction — deterministic main action ranker ─────────────────────
//
// Pure function that computes the "main action" for the right sidebar.
// The main action is the single highlighted call-to-action shown at the
// top of the sidebar when contextual confidence is high enough.
//
// ## Algorithm
// 1. For each section with a mainAction, compute a confidence score.
// 2. Matching type → confidence 1.0 (perfect match).
// 3. Mismatched type → apply type penalty (see below).
// 4. Suggestion flag (−0.3) penalizes lower-priority suggestions.
// 5. Winner: highest confidence >= THRESHOLD (0.7), tiebreak by priority.
// 6. Confidence < THRESHOLD → null (no main action shown).
//
// ## Confidence scoring
//   base = 1.0 when section's target type === selection type
//   base = 0.5 when section's target type === 'any'
//   base = 0.3 when section's target type !== selection type (type mismatch)
//   suggestion_penalty = −0.3 (sections flagged as suggestions)
//   result = base + suggestion_penalty, clamped to [0, 1]
//
// ## Tie-breaking
//   1. Higher confidence wins
//   2. Lower priority number wins (earlier in sidebar = more fundamental)

import type { Selection, BlockSelection, PageSelection, GraphSelection } from '../selection/types'
import type { RightSidebarSection, RankedAction } from './types'

/** Minimum confidence required to show a main action. */
export const MAIN_ACTION_THRESHOLD = 0.7

/**
 * A section can declare its main action type so the ranker can
 * score it appropriately.
 */
export type MainActionTargetType = 'block' | 'page' | 'any'

/**
 * Optional main action config a section can declare.
 */
export interface SectionMainAction {
  /** Which selection type this action is most relevant for. */
  targetType: MainActionTargetType
  /** Human-readable label for the action button. */
  label: string
  /** True if this is a secondary suggestion (penalized by −0.3). */
  suggestion?: boolean
  /** Callback when the action is clicked. */
  onClick?: () => void
}

/**
 * Compute the main action for the given selection from a list of sections.
 *
 * Returns null when no action meets the confidence threshold.
 * Returns RankedAction for the winning action.
 *
 * ## Pure function — no side effects
 * The function is fully deterministic given the same inputs.
 */
export function rankMainAction(
  selection: Selection,
  sections: readonly RightSidebarSection[],
  mainActions: ReadonlyMap<string, SectionMainAction>,
): RankedAction | null {
  // Null selection (graph context): no main action
  if (selection === null) return null

  const typeMap: Record<string, MainActionTargetType> = {
    block: 'block',
    page: 'page',
    graph: 'any',
  }

  const selectionType: MainActionTargetType = typeMap[selection.type] ?? 'any'

  let best: RankedAction | null = null

  for (const section of sections) {
    const action = mainActions.get(section.id)
    if (!action) continue

    let base: number
    if (action.targetType === selectionType) {
      base = 1.0
    } else if (action.targetType === 'any') {
      base = 0.5
    } else {
      // Type mismatch — section targets a different selection type
      base = 0.3
    }

    let confidence = base
    if (action.suggestion) {
      confidence -= 0.3
    }
    // Clamp to [0, 1]
    confidence = Math.max(0, Math.min(1, confidence))

    // Threshold gate
    if (confidence < MAIN_ACTION_THRESHOLD) continue

    // Tie-break: higher confidence wins, then lower priority (earlier section)
    if (
      best === null ||
      confidence > best.confidence ||
      (confidence === best.confidence && section.priority < getSectionPriority(best.sectionId, sections))
    ) {
      best = {
        sectionId: section.id,
        confidence,
        label: action.label,
        onClick: action.onClick,
      }
    }
  }

  return best
}

/**
 * Look up a section's priority by id.
 */
function getSectionPriority(id: string, sections: readonly RightSidebarSection[]): number {
  const section = sections.find((s) => s.id === id)
  return section?.priority ?? 999
}

/**
 * Build the main action map from section declarations.
 * Called once by the sidebar shell at mount time.
 */
export function buildMainActionMap(
  sections: readonly RightSidebarSection[],
  getMainAction: (sectionId: string) => SectionMainAction | undefined,
): ReadonlyMap<string, SectionMainAction> {
  const map = new Map<string, SectionMainAction>()
  for (const section of sections) {
    const action = getMainAction(section.id)
    if (action) {
      map.set(section.id, action)
    }
  }
  return map
}
