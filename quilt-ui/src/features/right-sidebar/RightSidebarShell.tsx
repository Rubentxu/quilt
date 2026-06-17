// ─── RightSidebarShell — unified right sidebar container ────────────────────
//
// Renders the contextual right sidebar: visible by default, collapsible,
// section tabs, main action slot, and empty state when no sections apply.
//
// ## Visibility
// - Visible by default (per ADR-0030 §14)
// - Toggle via LayoutMenu or keyboard shortcut (t r)
// - Persisted via PanelVisibilityContext ('right-sidebar' panel id)
//
// ## Layout
//   ┌─────────────────────────────┐
//   │  [tab1] [tab2] [tab3]       │  ← Section tab bar
//   ├─────────────────────────────┤
//   │  ┌─────────────────────┐    │
//   │  │  MAIN ACTION BTN   │    │  ← Ranked main action (or empty)
//   │  └─────────────────────────┘│
//   ├─────────────────────────────┤
//   │                             │
//   │  <Active section content>   │  ← First visible section
//   │                             │
//   └─────────────────────────────┘
//
// ## Self-contained design
// The shell reads registered sections from the registry directly.
// Sections register themselves via side-effect barrel imports (sections/index.ts).
// No props needed for section discovery.

import { useState, useMemo } from 'react'
import { usePanelVisibility } from '@features/dashboard'
import { useSelection } from './selection'
import { getVisibleSections, getSections } from './sections/registry'
import { RightSidebarEmptyState } from './RightSidebarEmptyState'
import type { RightSidebarSection } from './sections/types'
import type { Selection } from './selection/types'

/** Panel id used in PanelVisibilityContext */
export const RIGHT_SIDEBAR_PANEL_ID = 'right-sidebar' as const

interface RightSidebarShellProps {
  /** Optional override for which section is active (defaults to first visible) */
  activeSectionId?: string | null
  /** Called when the user switches sections */
  onSectionChange?: (sectionId: string) => void
}

export function RightSidebarShell({
  activeSectionId,
  onSectionChange,
}: RightSidebarShellProps) {
  const selection = useSelection()
  const { visiblePanels, togglePanel } = usePanelVisibility()
  const isOpen = visiblePanels.has(RIGHT_SIDEBAR_PANEL_ID)

  const allSections = useMemo((): readonly RightSidebarSection[] => getSections(), [])
  const visibleSections = useMemo(
    () => getVisibleSections(selection),
    [selection, allSections],
  )

  // Derive the active section
  const activeSection = useMemo((): RightSidebarSection | null => {
    if (activeSectionId) {
      return visibleSections.find((s: RightSidebarSection) => s.id === activeSectionId) ?? null
    }
    return visibleSections[0] ?? null
  }, [activeSectionId, visibleSections])

  function handleTabClick(sectionId: string) {
    onSectionChange?.(sectionId)
  }

  if (!isOpen) return null

  return (
    <aside
      data-testid="right-sidebar"
      style={{
        width: '320px',
        borderLeft: '1px solid var(--color-border)',
        background: 'var(--color-surface)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        overflow: 'hidden',
      }}
    >
      {/* Tab bar */}
      {visibleSections.length > 1 && (
        <div
          role="tablist"
          data-testid="right-sidebar-tabs"
          style={{
            display: 'flex',
            borderBottom: '1px solid var(--color-border)',
            padding: '0 var(--space-2)',
            gap: 'var(--space-1)',
            overflowX: 'auto',
          }}
        >
          {visibleSections.map((section: RightSidebarSection) => (
            <button
              key={section.id}
              role="tab"
              aria-selected={activeSection?.id === section.id}
              data-testid={`right-sidebar-tab-${section.id}`}
              onClick={() => handleTabClick(section.id)}
              style={{
                padding: 'var(--space-2) var(--space-3)',
                border: 'none',
                background: 'transparent',
                borderBottom:
                  activeSection?.id === section.id
                    ? '2px solid var(--color-accent)'
                    : '2px solid transparent',
                color:
                  activeSection?.id === section.id
                    ? 'var(--color-text-primary)'
                    : 'var(--color-text-muted)',
                fontSize: '12px',
                fontWeight: 500,
                cursor: 'pointer',
                whiteSpace: 'nowrap',
              }}
            >
              {section.label}
            </button>
          ))}
        </div>
      )}

      {/* Main action slot — per ADR-0030 §14: max 0 or 1 action when confidence >= 0.7
          TODO(GS-8): wire section mainAction declarations to rankMainAction */}
      <div
        data-testid="right-sidebar-main-action"
        style={{
          padding: 'var(--space-3) var(--space-4)',
          borderBottom: '1px solid var(--color-border)',
        }}
      >
        <div
          style={{
            padding: 'var(--space-2)',
            textAlign: 'center',
            fontSize: '12px',
            color: 'var(--color-text-disabled)',
          }}
        />
      </div>

      {/* Section content */}
      <div
        style={{
          flex: 1,
          overflow: 'auto',
        }}
      >
        {activeSection ? (
          (() => {
            const Component = activeSection.component
            return (
              <Component selection={selection} />
            )
          })()
        ) : (
          <RightSidebarEmptyState />
        )}
      </div>
    </aside>
  )
}
