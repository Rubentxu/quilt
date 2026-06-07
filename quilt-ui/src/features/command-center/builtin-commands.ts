import toast from 'react-hot-toast'
import { CommandCategory, type Command } from './types'

/**
 * Build the built-in command set: navigation, view toggles, capture,
 * and help.
 *
 * The returned commands are LOCAL (`target: 'local'`) and do not
 * need any external state — they capture their dependencies via
 * closure when the provider calls this factory. The provider runs
 * the factory once on mount and registers every command.
 *
 * Factory function (not a constant) so future enhancements (custom
 * prompts, locale-aware labels) can pass arguments without
 * touching the call site.
 *
 * The 9-command set is verified by `builtin-commands.test.ts`:
 *   5 Navigation  (nav/home, nav/journal, nav/graph, nav/pages, nav/settings)
 *   2 View       (view/toggle-theme, view/toggle-sidebar)
 *   1 Capture    (capture/quick)
 *   1 Help       (help/shortcuts)
 */
export function createBuiltinCommands(): Command[] {
  // ──── Navigation (priority 10) ─────────────────────────────────
  // Lower priority numbers surface first when the modal is open
  // with an empty query — Navigation wins so users can land
  // somewhere useful in two keystrokes.

  const navHome: Command = {
    id: 'nav/home',
    label: 'Go to Home',
    category: CommandCategory.Navigation,
    priority: 10,
    target: 'local',
    execute: ({ navigate }) => {
      navigate({ to: '/' })
    },
  }

  const navJournal: Command = {
    id: 'nav/journal',
    label: "Go to Today's Journal",
    category: CommandCategory.Navigation,
    priority: 10,
    target: 'local',
    execute: ({ navigate }) => {
      const today = new Date().toISOString().split('T')[0]
      navigate({ to: '/journal/$date', params: { date: today } })
    },
  }

  const navGraph: Command = {
    id: 'nav/graph',
    label: 'Go to Graph',
    category: CommandCategory.Navigation,
    priority: 10,
    target: 'local',
    execute: ({ navigate }) => {
      navigate({ to: '/graph' })
    },
  }

  const navPages: Command = {
    id: 'nav/pages',
    label: 'Go to All Pages',
    category: CommandCategory.Navigation,
    priority: 10,
    target: 'local',
    execute: ({ navigate }) => {
      navigate({ to: '/pages' })
    },
  }

  const navSettings: Command = {
    id: 'nav/settings',
    label: 'Go to Settings',
    category: CommandCategory.Navigation,
    priority: 10,
    target: 'local',
    execute: ({ navigate }) => {
      navigate({ to: '/settings' })
    },
  }

  // ──── View (priority 20) ──────────────────────────────────────

  const viewToggleTheme: Command = {
    id: 'view/toggle-theme',
    label: 'Toggle Dark Mode',
    category: CommandCategory.View,
    priority: 20,
    target: 'local',
    execute: () => {
      const html = document.documentElement
      const current = html.getAttribute('data-theme') === 'dark' ? 'dark' : 'light'
      const next = current === 'dark' ? 'light' : 'dark'
      html.setAttribute('data-theme', next)
      try {
        localStorage.setItem('quilt-theme', next)
      } catch {
        // localStorage may be unavailable (private mode / quota).
        // The in-memory attribute change still applies for this
        // session; the persistence is best-effort.
      }
    },
  }

  // The sidebar lives as React state inside AppShell. The palette
  // command is a no-op stub for Phase 1 — the existing kebab menu
  // and the leader-key `g s` shortcut still drive the toggle.
  // Wiring this to the AppShell state requires either a context
  // or a custom event; both are follow-ups tracked separately.
  const viewToggleSidebar: Command = {
    id: 'view/toggle-sidebar',
    label: 'Toggle Sidebar',
    category: CommandCategory.View,
    priority: 20,
    target: 'local',
    execute: () => {
      // Phase 1: no global sidebar store. Surfacing a toast gives
      // the user feedback that the command registered but doesn't
      // have a real effect yet, so they don't think the palette is
      // broken.
      toast('Sidebar toggle coming from the kebab menu for now', {
        icon: 'ℹ️',
      })
    },
  }

  // ──── Capture (priority 30) ───────────────────────────────────

  const captureQuick: Command = {
    id: 'capture/quick',
    label: 'Quick Capture',
    category: CommandCategory.Capture,
    priority: 30,
    target: 'local',
    execute: async ({ api }) => {
      const content = window.prompt('Capture:')
      if (!content || !content.trim()) return
      const today = new Date().toISOString().split('T')[0]
      try {
        await api.createBlock({ pageName: today, content: content.trim() })
        toast.success('Captured to today\'s journal')
      } catch (err) {
        toast.error('Failed to capture block')
        // Surface the error to the dev console; the toaster is
        // the user-facing channel. Don't throw — the palette
        // already closed and the user has moved on.
        // eslint-disable-next-line no-console
        console.error('Quick capture failed:', err)
      }
    },
  }

  // ──── Help (priority 40) ──────────────────────────────────────

  const helpShortcuts: Command = {
    id: 'help/shortcuts',
    label: 'Keyboard Shortcuts',
    category: CommandCategory.Help,
    priority: 40,
    target: 'local',
    execute: () => {
      // The full shortcuts panel is mounted inside the AppShell's
      // FloatingHelpButton; showing a toast here is the lightweight
      // hint that the command registered. Users who want the
      // full list can still hit the kebab menu → "Keyboard
      // shortcuts".
      toast(
        'Cmd/Ctrl+K — search · Cmd/Ctrl+Shift+K — command palette · g then h/j/p/g — navigate',
        { duration: 5000 },
      )
    },
  }

  return [
    navHome,
    navJournal,
    navGraph,
    navPages,
    navSettings,
    viewToggleTheme,
    viewToggleSidebar,
    captureQuick,
    helpShortcuts,
  ]
}
