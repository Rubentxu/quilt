import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useEffect, useState } from 'react'
import {
  CommandRegistryProvider,
  useCommandRegistry,
} from '../context'
import { CommandCenter } from '../CommandCenter'
import { CommandCategory, type Command } from '../types'

/**
 * End-to-end integration test for the CommandRegistry wiring.
 *
 * Real features (AppShell) wire three things together:
 *   1. `CommandRegistryProvider` at the root.
 *   2. A keyboard listener that toggles `isOpen` on Cmd/Ctrl+Shift+K.
 *   3. The `<CommandCenter>` modal, mounted only while open.
 *
 * This test renders a small harness that mirrors the same
 * structure, then drives the full flow: fire the global shortcut,
 * type a query, click a result, and assert the command's `execute`
 * was called.
 *
 * The harness uses the real `useCommandRegistry` (no mocks) so
 * the integration boundary is the actual `useEffect` chain — the
 * same one that runs in production.
 */

// ──── Test router stub ──────────────────────────────────────────
//
// The provider calls `useNavigate()` to build the CommandContext.
// jsdom has no real router; we stub it so navigation is observable
// via a vi.fn().
const mockNavigate = vi.fn()
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// ──── Test commands ─────────────────────────────────────────────
//
// Pre-registered via `TestRegistrar` (a tiny useEffect-based
// component) so the modal has a known surface to query against.
// We deliberately use distinct ids from the builtin command set
// (prefix `integration/`) so the assertions are stable across
// refactors of the builtin factory.

const INTEGRATION_COMMANDS: Command[] = [
  {
    id: 'integration/go-home',
    label: 'Integration: Go Home',
    category: CommandCategory.Navigation,
    priority: 10,
    target: 'local',
    execute: vi.fn(),
  },
  {
    id: 'integration/toggle-theme',
    label: 'Integration: Toggle Theme',
    category: CommandCategory.View,
    priority: 20,
    target: 'local',
    execute: vi.fn(),
  },
]

function TestRegistrar({ commands }: { commands: Command[] }) {
  const { register, unregister } = useCommandRegistry()
  useEffect(() => {
    for (const c of commands) register(c)
    return () => {
      for (const c of commands) unregister(c.id)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [register, unregister])
  return null
}

// ──── Test harness ─────────────────────────────────────────────
//
// Mirrors the AppShell wiring: keyboard listener + modal mount.
// Kept inline so the test reads as a single, self-contained flow.

function Harness() {
  const [open, setOpen] = useState(false)
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Mirrors the AppShell handler: Cmd/Ctrl+Shift+K toggles
      // the palette.
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'K' || e.key === 'k')) {
        e.preventDefault()
        setOpen((prev) => !prev)
      }
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [])

  return (
    <CommandRegistryProvider>
      <TestRegistrar commands={INTEGRATION_COMMANDS} />
      {open && <CommandCenter isOpen={open} onClose={() => setOpen(false)} />}
    </CommandRegistryProvider>
  )
}

beforeEach(() => {
  mockNavigate.mockReset()
  for (const c of INTEGRATION_COMMANDS) {
    if (typeof c.execute === 'function' && 'mockClear' in c.execute) {
      ;(c.execute as ReturnType<typeof vi.fn>).mockClear()
    }
  }
})

afterEach(() => {
  cleanup()
})

// ──── Tests ────────────────────────────────────────────────────

describe('CommandRegistry integration', () => {
  it('opens the modal when the user fires Cmd+Shift+K globally', async () => {
    render(<Harness />)

    // Closed by default — no input visible.
    expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()

    // Simulate the user hitting Cmd+Shift+K on the document.
    fireEvent.keyDown(document, { key: 'K', ctrlKey: true, shiftKey: true })

    // The modal mounts and focuses the input.
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/type a command/i)).toBeInTheDocument()
    })
  })

  it('opens with Meta+Shift+K (macOS variant)', async () => {
    render(<Harness />)

    fireEvent.keyDown(document, { key: 'K', metaKey: true, shiftKey: true })

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/type a command/i)).toBeInTheDocument()
    })
  })

  it('does NOT open on a plain K press (no modifiers)', () => {
    render(<Harness />)
    fireEvent.keyDown(document, { key: 'K' })
    // The palette is still closed — only Cmd/Ctrl+Shift+K opens it.
    expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()
  })

  it('does NOT open on Cmd+K without Shift (that is the SearchModal shortcut)', () => {
    render(<Harness />)
    fireEvent.keyDown(document, { key: 'K', metaKey: true })
    // No modal — that's the legacy SearchModal's keybind.
    expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()
  })

  it('toggles closed on a second Cmd+Shift+K', async () => {
    render(<Harness />)

    // Open.
    fireEvent.keyDown(document, { key: 'K', ctrlKey: true, shiftKey: true })
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/type a command/i)).toBeInTheDocument()
    })

    // Close.
    fireEvent.keyDown(document, { key: 'K', ctrlKey: true, shiftKey: true })
    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()
    })
  })

  it('end-to-end: open via keyboard → type → click → execute runs', async () => {
    const user = userEvent.setup()
    render(<Harness />)

    // 1. Open the palette via the global shortcut.
    fireEvent.keyDown(document, { key: 'K', ctrlKey: true, shiftKey: true })
    const input = await screen.findByPlaceholderText(/type a command/i)

    // 2. Type a query that matches one of the integration commands.
    await user.type(input, 'go home')

    // 3. The matching row appears in the list.
    const row = await screen.findByText('Integration: Go Home')
    expect(row).toBeInTheDocument()

    // 4. Click the row.
    await user.click(row)

    // 5. The command's `execute` was called and the modal closed.
    const home = INTEGRATION_COMMANDS.find((c) => c.id === 'integration/go-home')!
    expect(home.execute).toHaveBeenCalledTimes(1)
    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()
    })
  })

  it('end-to-end: open via keyboard → type → Enter on a result → execute runs', async () => {
    const user = userEvent.setup()
    render(<Harness />)

    fireEvent.keyDown(document, { key: 'K', ctrlKey: true, shiftKey: true })
    const input = await screen.findByPlaceholderText(/type a command/i)

    // Type a query that matches the toggle-theme command. The
    // label is 'Integration: Toggle Theme' — we query just
    // 'theme' so we don't have to match the colon character.
    await user.type(input, 'theme')

    await waitFor(() => {
      expect(screen.getByText('Integration: Toggle Theme')).toBeInTheDocument()
    })

    fireEvent.keyDown(input, { key: 'Enter' })

    const toggle = INTEGRATION_COMMANDS.find(
      (c) => c.id === 'integration/toggle-theme',
    )!
    expect(toggle.execute).toHaveBeenCalledTimes(1)
    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()
    })
  })

  it('reopening the palette starts with an empty query (no carryover from the previous session)', async () => {
    const user = userEvent.setup()
    render(<Harness />)

    // Open, type something, close via Escape.
    fireEvent.keyDown(document, { key: 'K', ctrlKey: true, shiftKey: true })
    const input = (await screen.findByPlaceholderText(/type a command/i)) as HTMLInputElement
    await user.type(input, 'integration')

    fireEvent.keyDown(input, { key: 'Escape' })
    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()
    })

    // Reopen.
    fireEvent.keyDown(document, { key: 'K', ctrlKey: true, shiftKey: true })
    const fresh = (await screen.findByPlaceholderText(/type a command/i)) as HTMLInputElement
    expect(fresh.value).toBe('')
  })
})
