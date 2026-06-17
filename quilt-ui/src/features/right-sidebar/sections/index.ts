// ─── sections/index — RightSidebarSection barrel (side-effect imports) ─────
//
// CRITICAL: This barrel is the SOLE import site for feature panels.
// The registry is a LEAF module that must NOT import features directly.
// Only this barrel imports features (to register them), keeping the
// registry cycle-free.
//
// Registration order determines tie-breaking in the ranker when priorities
// are equal (earlier registration wins).
//
// ## No logic here — pure re-exports only
// If this barrel had logic that depended on imported values, the imports
// would be evaluated eagerly. By keeping it as pure re-exports, the
// registration side effects in the imported modules run at import time.

import { registerSection } from './registry'

// ─── Backlinks ───────────────────────────────────────────────────────────────

import { backlinksSection } from './backlinks/BacklinksSection'

registerSection(backlinksSection)

// ─── Block Properties (block-scoped, priority 200) ─────────────────────────

import { blockPropertiesSection } from './block-properties/BlockPropertiesSection'

registerSection(blockPropertiesSection)

// ─── Cognitive Panels (priority 300-370) ───────────────────────────────────

import {
  agentActivitySection,
  agentRoomSection,
  structuralGraphSection,
  semanticInsightSection,
  cognitiveGraphSection,
  decayMonitorSection,
  weeklyReviewSection,
  serendipitySection,
} from './cognitive/CognitiveSections'

registerSection(agentActivitySection)
registerSection(agentRoomSection)
registerSection(structuralGraphSection)
registerSection(semanticInsightSection)
registerSection(cognitiveGraphSection)
registerSection(decayMonitorSection)
registerSection(weeklyReviewSection)
registerSection(serendipitySection)

// ─── Migration / Import (GS-9, priority 400, utility) ───────────────────────

import { migrationSection } from './import/ImportSection'

registerSection(migrationSection)

// ─── Re-export public API ───────────────────────────────────────────────────

export { getSections, getVisibleSections, registerSection, isRegistered } from './registry'
export type { RightSidebarSection, SectionPriority, SectionPredicate, RankedAction } from './types'
export { rankMainAction, MAIN_ACTION_THRESHOLD, buildMainActionMap } from './rankMainAction'
export type { SectionMainAction, MainActionTargetType } from './rankMainAction'
