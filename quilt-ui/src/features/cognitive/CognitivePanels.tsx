// ─── CognitivePanels — right-side container for the cognitivo:: family ──
//
// Renders the three cognitive panels as a single right-side column:
//
//   - AgentActivityFeed (always available; reads from the global
//                        `agent-activity` panel flag).
//   - StructuralGraph   (page-scoped; reads from the current page
//                        name).
//   - SemanticInsight   (page-scoped; reads from the current page
//                        name).
//
// Each panel honours its own flag in `PanelVisibilityContext` —
// toggling a panel via the CommandRegistry (e.g.
// `cog/toggle-structural-graph`) shows/hides just that section
// without affecting the others.
//
// On mobile, the column collapses and the panels are hidden — the
// user can still access them via the layout menu and the command
// palette. Mobile-specific cognitive UX is a follow-up; the
// BacklinksPanel has its own bottom-sheet treatment that we'll
// mirror when the user demand is there.

import { usePanelVisibility } from '@features/dashboard'
import { AgentActivityFeed } from './AgentActivityFeed'
import { StructuralGraph } from './StructuralGraph'
import { SemanticInsight } from './SemanticInsight'

interface CognitivePanelsProps {
  pageName: string | null
}

export function CognitivePanels({ pageName }: CognitivePanelsProps) {
  const { visiblePanels } = usePanelVisibility()

  const showAgentActivity = visiblePanels.has('agent-activity')
  const showStructural = visiblePanels.has('structural-graph')
  const showSemantic = visiblePanels.has('semantic-insight')

  // Skip the entire column if no cognitive panel is enabled — saves
  // a column of dead space.
  if (!showAgentActivity && !showStructural && !showSemantic) return null

  return (
    <aside
      data-testid="cognitive-panels"
      style={{
        width: '320px',
        borderLeft: '1px solid var(--color-border)',
        background: 'var(--color-surface)',
        overflow: 'auto',
        flexShrink: 0,
        boxShadow: 'var(--shadow-sm)',
      }}
    >
      {showAgentActivity && (
        <section
          data-testid="cognitive-panel-agent-activity"
          style={{ borderBottom: '1px solid var(--color-border)' }}
        >
          <AgentActivityFeed maxItems={15} />
        </section>
      )}
      {showStructural && (
        <section
          data-testid="cognitive-panel-structural-graph"
          style={{ borderBottom: '1px solid var(--color-border)' }}
        >
          <StructuralGraph pageName={pageName} isOpen={showStructural} />
        </section>
      )}
      {showSemantic && (
        <section data-testid="cognitive-panel-semantic-insight">
          <SemanticInsight pageName={pageName} isOpen={showSemantic} />
        </section>
      )}
    </aside>
  )
}
