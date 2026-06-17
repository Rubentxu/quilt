// ─── sections/cognitive/CognitiveSections ──────────────────────────────────
//
// Each CognitivePanel migrated as an individual RightSidebarSection.
// Priority range: 300-370 (cognitive family)
//
// Migration: CognitivePanels was previously a monolithic column with
// 8 visibility flags. Each panel is now an independent section.
// The monolithic CognitivePanels.tsx is retired.

import { memo } from 'react'
import {
  AgentActivityFeed,
  StructuralGraph,
  SemanticInsight,
  DecayMonitor,
  WeeklyReview,
} from '@features/cognitive'
import { AgentRoom } from '@features/agent-room'
import type { RightSidebarSection } from '../types'
import type { Selection } from '../../selection/types'

// ─── Shared predicate helpers ───────────────────────────────────────────────

function isPageScope(selection: Selection): boolean {
  return selection?.type === 'page' || selection?.type === 'block'
}

function getPageName(selection: Selection): string | null {
  if (selection?.type === 'page') return selection.pageName
  if (selection?.type === 'block') return selection.pageName
  return null
}

// ─── Agent Activity Section ──────────────────────────────────────────────────

const AgentActivitySectionComponent = memo(function AgentActivitySectionComponent() {
  return <AgentActivityFeed maxItems={15} />
})

export const AGENT_ACTIVITY_SECTION_ID = 'agent-activity'

export const agentActivitySection: RightSidebarSection = {
  id: AGENT_ACTIVITY_SECTION_ID,
  label: 'Agent Activity',
  priority: 300,
  visible: true,
  component: AgentActivitySectionComponent,
}

// ─── Agent Room Section ─────────────────────────────────────────────────────

const AgentRoomSectionComponent = memo(function AgentRoomSectionComponent() {
  return <AgentRoom />
})

export const AGENT_ROOM_SECTION_ID = 'agent-room'

export const agentRoomSection: RightSidebarSection = {
  id: AGENT_ROOM_SECTION_ID,
  label: 'Agent Room',
  priority: 305,
  visible: true,
  component: AgentRoomSectionComponent,
}

// ─── Structural Graph Section ──────────────────────────────────────────────

const StructuralGraphSectionComponent = memo(function StructuralGraphSectionComponent({ selection }: { selection: Selection }) {
  return <StructuralGraph pageName={getPageName(selection)} isOpen={true} />
})

export const STRUCTURAL_GRAPH_SECTION_ID = 'structural-graph'

export const structuralGraphSection: RightSidebarSection = {
  id: STRUCTURAL_GRAPH_SECTION_ID,
  label: 'Structure',
  priority: 310,
  visible: true,
  predicate: isPageScope,
  component: StructuralGraphSectionComponent,
}

// ─── Semantic Insight Section ───────────────────────────────────────────────

const SemanticInsightSectionComponent = memo(function SemanticInsightSectionComponent({ selection }: { selection: Selection }) {
  return <SemanticInsight pageName={getPageName(selection)} isOpen={true} />
})

export const SEMANTIC_INSIGHT_SECTION_ID = 'semantic-insight'

export const semanticInsightSection: RightSidebarSection = {
  id: SEMANTIC_INSIGHT_SECTION_ID,
  label: 'Insights',
  priority: 320,
  visible: true,
  predicate: isPageScope,
  component: SemanticInsightSectionComponent,
}

// ─── Cognitive Graph Section (placeholder — not yet implemented) ─────────────

const CognitiveGraphSectionComponent = memo(function CognitiveGraphSectionComponent() {
  return (
    <div style={{ padding: 'var(--space-4)', fontSize: '12px', color: 'var(--color-text-muted)' }}>
      Cognitive graph coming soon
    </div>
  )
})

export const COGNITIVE_GRAPH_SECTION_ID = 'cognitive-graph'

export const cognitiveGraphSection: RightSidebarSection = {
  id: COGNITIVE_GRAPH_SECTION_ID,
  label: 'Cognitive Graph',
  priority: 330,
  visible: false, // Not yet implemented
  component: CognitiveGraphSectionComponent,
}

// ─── Decay Monitor Section ──────────────────────────────────────────────────

const DecayMonitorSectionComponent = memo(function DecayMonitorSectionComponent() {
  return <DecayMonitor />
})

export const DECAY_MONITOR_SECTION_ID = 'decay-monitor'

export const decayMonitorSection: RightSidebarSection = {
  id: DECAY_MONITOR_SECTION_ID,
  label: 'Decay Monitor',
  priority: 340,
  visible: true,
  component: DecayMonitorSectionComponent,
}

// ─── Weekly Review Section ──────────────────────────────────────────────────

const WeeklyReviewSectionComponent = memo(function WeeklyReviewSectionComponent() {
  return <WeeklyReview />
})

export const WEEKLY_REVIEW_SECTION_ID = 'weekly-review'

export const weeklyReviewSection: RightSidebarSection = {
  id: WEEKLY_REVIEW_SECTION_ID,
  label: 'Weekly Review',
  priority: 350,
  visible: true,
  component: WeeklyReviewSectionComponent,
}

// ─── Serendipity Section (placeholder — not yet implemented) ──────────────

const SerendipitySectionComponent = memo(function SerendipitySectionComponent() {
  return (
    <div style={{ padding: 'var(--space-4)', fontSize: '12px', color: 'var(--color-text-muted)' }}>
      Serendipity feed coming soon
    </div>
  )
})

export const SERENDIPITY_SECTION_ID = 'serendipity'

export const serendipitySection: RightSidebarSection = {
  id: SERENDIPITY_SECTION_ID,
  label: 'Serendipity',
  priority: 360,
  visible: false, // Not yet implemented
  component: SerendipitySectionComponent,
}

// ─── Cognitive section list ─────────────────────────────────────────────────

export const COGNITIVE_SECTIONS: readonly RightSidebarSection[] = [
  agentActivitySection,
  agentRoomSection,
  structuralGraphSection,
  semanticInsightSection,
  cognitiveGraphSection,
  decayMonitorSection,
  weeklyReviewSection,
  serendipitySection,
]
