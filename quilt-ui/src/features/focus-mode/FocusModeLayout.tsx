// ─── FocusModeLayout — applies focus mode visual layout ───────────
//
// Wraps the editor area and applies focus mode styles:
//   - Sidebar is hidden (handled externally via PanelVisibilityContext)
//   - Editor is centered in a max-width column
//   - Font size is increased
//   - AI panel slides in from the right when isAIPanelOpen
//
// The layout component itself does NOT control panel visibility —
// the parent is responsible for hiding the sidebar when
// `isActive` is true.

import type { ReactNode } from 'react'
import { useFocusMode } from './FocusModeContext'
import { AIPanel } from './AIPanel'

interface FocusModeLayoutProps {
  children: ReactNode
}

/**
 * Focus mode layout wrapper. Pass the editor content as children.
 * When focus mode is active, applies centered column layout with
 * larger font. When the AI panel is open, renders the AIPanel
 * to the right (slide-in).
 */
export function FocusModeLayout({ children }: FocusModeLayoutProps) {
  const { isActive, isAIPanelOpen } = useFocusMode()

  if (!isActive) {
    return <>{children}</>
  }

  return (
    <div
      data-testid="focus-mode-layout"
      style={{
        display: 'flex',
        flexDirection: 'row',
        width: '100%',
        height: '100%',
        position: 'relative',
      }}
    >
      {/* Main editor area — centered with larger font */}
      <div
        data-testid="focus-mode-editor"
        style={{
          flex: 1,
          display: 'flex',
          justifyContent: 'center',
          padding: '0 var(--space-8)',
          overflow: 'auto',
          // Larger font in focus mode
          fontSize: '18px',
          lineHeight: 1.7,
        }}
      >
        <div
          style={{
            width: '100%',
            maxWidth: '720px',
            padding: 'var(--space-12) 0',
          }}
        >
          {children}
        </div>
      </div>

      {/* AI panel — slides in from right */}
      {isAIPanelOpen && (
        <AIPanel />
      )}
    </div>
  )
}
