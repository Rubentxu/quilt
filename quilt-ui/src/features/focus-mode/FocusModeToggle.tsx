// ─── FocusModeToggle — keyboard shortcut handler + visual indicator
//
// Registers Cmd+. as the global toggle shortcut for focus mode.
// Shows a subtle visual indicator when focus mode is active.

import { useKeyboardShortcuts } from '@shared/hooks/useKeyboardShortcuts'
import { useFocusMode } from './FocusModeContext'

/**
 * Handles the Cmd+. keyboard shortcut to toggle focus mode.
 * Also wires Escape to exit focus mode.
 *
 * This component is a no-op when rendered outside a
 * FocusModeProvider — the context consumer returns safe defaults.
 */
export function FocusModeToggle() {
  const { isActive, toggle, setActive } = useFocusMode()

  useKeyboardShortcuts({
    'Cmd+.': toggle,
    // Also support Ctrl+. on non-Mac platforms
    'Ctrl+.': toggle,
    Escape: () => {
      if (isActive) setActive(false)
    },
  })

  // Visual indicator: nothing to render, the shortcut is the point.
  // The parent AppShell or layout can read `isActive` from the
  // context to style themselves accordingly.
  return null
}
