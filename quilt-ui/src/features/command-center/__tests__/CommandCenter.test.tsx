import { render, screen, fireEvent, act, cleanup, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { CommandCenter } from '../CommandCenter'
import {
  CommandRegistryProvider,
  useCommandRegistry,
} from '../context'
import { CommandCategory, type Command } from '../types'
import { useEffect } from 'react'
import type { ReactNode } from 'react'

/**
 * Tests for the CommandCenter modal.
 *
 * The modal wires three things together: the input, the registry
 * (via `useCommandRegistry`), and the keyboard navigation. We mock
 * `useNavigate` (no real router in jsdom) and pre-load the registry
 * with a small set of commands so we can assert label and category
 * rendering without coupling to the builtin list.
 */

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

/** Extra commands registered for the modal tests — independent
 *  of the builtin set so the assertions are stable across refactors. */
const EXTRA_COMMANDS: Command[] = [
  {
    id: 'test/nav-home',
    label: 'Test Home',
    category: CommandCategory.Navigation,
    priority: 10,
    target: 'local',
    execute: vi.fn(),
  },
  {
    id: 'test/toggle-theme',
    label: 'Test Toggle Theme',
    category: CommandCategory.View,
    priority: 20,
    shortcut: 'Ctrl+Shift+T',
    target: 'local',
    execute: vi.fn(),
  },
  {
    id: 'test/quick-capture',
    label: 'Test Quick Capture',
    category: CommandCategory.Capture,
    priority: 30,
    target: 'local',
    execute: vi.fn(),
  },
  {
    id: 'test/keyboard-help',
    label: 'Test Keyboard Help',
    category: CommandCategory.Help,
    priority: 40,
    target: 'local',
    execute: vi.fn(),
  },
]

/** Wrapper that pre-registers the test commands. Tests can then
 *  assert against just the test/ ids without depending on the
 *  built-in command surface. */
function HarnessWithCommands({
  children,
  isOpen = true,
}: {
  children?: ReactNode
  isOpen?: boolean
}) {
  return (
    <CommandRegistryProvider>
      <TestRegistrar commands={EXTRA_COMMANDS} />
      <CommandCenter isOpen={isOpen} onClose={vi.fn()} />
    </CommandRegistryProvider>
  )
}

/** Registers the test commands on mount and unregisters on unmount.
 *  Uses the registry hook directly so the assertions are about the
 *  modal's behavior, not the registration flow (that has its own
 *  coverage in context.test.tsx).
 *
 *  We use `useEffect` (not a render-time side effect) so React
 *  Testing Library's `act` flushes the registration before the
 *  test's first assertion. The registry dedups by id, so calling
 *  `register` on every mount is safe across re-mounts and avoids
 *  mutating the module-level command objects (which would leak
 *  state between tests).
 */
function TestRegistrar({ commands }: { commands: Command[] }) {
  const { register, unregister } = useCommandRegistry()
  useEffect(() => {
    for (const c of commands) {
      register(c)
    }
    return () => {
      for (const c of commands) {
        unregister(c.id)
      }
    }
    // commands is module-level (EXTRA_COMMANDS); the effect's
    // identity never changes across the test, so we can list
    // its fields as deps and avoid re-running on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [register, unregister])
  return null
}

beforeEach(() => {
  mockNavigate.mockReset()
  for (const c of EXTRA_COMMANDS) {
    if (typeof c.execute === 'function' && 'mockClear' in c.execute) {
      ;(c.execute as ReturnType<typeof vi.fn>).mockClear()
    }
  }
})

afterEach(() => {
  cleanup()
})

describe('CommandCenter', () => {
  it('renders nothing when isOpen is false', () => {
    render(
      <CommandRegistryProvider>
        <CommandCenter isOpen={false} onClose={vi.fn()} />
      </CommandRegistryProvider>,
    )
    // No overlay, no input — the modal is fully unmounted when
    // closed so it doesn't intercept keyboard events.
    expect(screen.queryByPlaceholderText(/type a command/i)).not.toBeInTheDocument()
  })

  it('renders the input and all commands when opened with an empty query', async () => {
    render(<HarnessWithCommands />)

    // TestRegistrar uses useEffect for registration; the test
    // commands appear after the first commit. `findByText` waits
    // for the async registration to settle.
    expect(await screen.findByText('Test Home')).toBeInTheDocument()
    expect(await screen.findByText('Test Toggle Theme')).toBeInTheDocument()
    expect(await screen.findByText('Test Quick Capture')).toBeInTheDocument()
    expect(await screen.findByText('Test Keyboard Help')).toBeInTheDocument()
  })

  it('filters results when the user types a query', async () => {
    const user = userEvent.setup()
    render(<HarnessWithCommands />)
    const input = screen.getByPlaceholderText(/type a command/i)

    await user.type(input, 'home')

    // Only commands whose label (or category) contains "home" survive.
    await waitFor(() => {
      expect(screen.getByText('Test Home')).toBeInTheDocument()
    })
    expect(screen.queryByText('Test Quick Capture')).not.toBeInTheDocument()
    expect(screen.queryByText('Test Keyboard Help')).not.toBeInTheDocument()
  })

  it('shows an empty state when no command matches the query', async () => {
    const user = userEvent.setup()
    render(<HarnessWithCommands />)
    const input = screen.getByPlaceholderText(/type a command/i)

    await user.type(input, 'zzzz-no-match')

    // Wait for the empty-state copy to appear.
    await waitFor(() => {
      expect(screen.getByText(/no commands match/i)).toBeInTheDocument()
    })
  })

  it('renders the category badge for each result', async () => {
    render(<HarnessWithCommands />)
    // Each command shows its category as a badge label. Use
    // findAllByText to wait for the async registration.
    expect((await screen.findAllByText('Navigation')).length).toBeGreaterThan(0)
    expect((await screen.findAllByText('View')).length).toBeGreaterThan(0)
    expect((await screen.findAllByText('Capture')).length).toBeGreaterThan(0)
    expect((await screen.findAllByText('Help')).length).toBeGreaterThan(0)
  })

  it('renders the optional shortcut badge when the command declares one', async () => {
    render(<HarnessWithCommands />)
    expect(await screen.findByText('Ctrl+Shift+T')).toBeInTheDocument()
  })

  it('ArrowDown / ArrowUp move the highlighted row', async () => {
    const user = userEvent.setup()
    render(<HarnessWithCommands />)
    const input = screen.getByPlaceholderText(/type a command/i)

    // Type a query that matches at least 2 of the test commands.
    await user.type(input, 'test')

    // Wait for the filtered list to render.
    await waitFor(() => {
      expect(screen.getAllByText(/Test/).length).toBeGreaterThanOrEqual(2)
    })

    // Default: the first row is highlighted (aria-selected=true).
    // ArrowDown → second row. ArrowDown → third. ArrowUp → second.
    const rows = screen.getAllByRole('option')
    expect(rows.length).toBeGreaterThanOrEqual(2)

    expect(rows[0]).toHaveAttribute('aria-selected', 'true')
    fireEvent.keyDown(input, { key: 'ArrowDown' })
    expect(rows[0]).toHaveAttribute('aria-selected', 'false')
    expect(rows[1]).toHaveAttribute('aria-selected', 'true')

    fireEvent.keyDown(input, { key: 'ArrowUp' })
    expect(rows[0]).toHaveAttribute('aria-selected', 'true')
    expect(rows[1]).toHaveAttribute('aria-selected', 'false')
  })

  it('Enter on a highlighted row calls execute and closes the modal', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <CommandRegistryProvider>
        <TestRegistrar commands={EXTRA_COMMANDS} />
        <CommandCenter isOpen={true} onClose={onClose} />
      </CommandRegistryProvider>,
    )
    const input = screen.getByPlaceholderText(/type a command/i)

    // Type a query that matches ONLY the test command. The builtin
    // 'Quick Capture' would also match a bare 'quick capture' query,
    // so we use the 'test' discriminator.
    await user.type(input, 'test quick')

    await waitFor(() => {
      expect(screen.getByText('Test Quick Capture')).toBeInTheDocument()
    })

    fireEvent.keyDown(input, { key: 'Enter' })

    // The matching command's `execute` was called.
    const capture = EXTRA_COMMANDS.find((c) => c.id === 'test/quick-capture')!
    expect(capture.execute).toHaveBeenCalledTimes(1)
    // The modal closed.
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('Escape closes the modal', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <CommandRegistryProvider>
        <CommandCenter isOpen={true} onClose={onClose} />
      </CommandRegistryProvider>,
    )

    const input = screen.getByPlaceholderText(/type a command/i)
    fireEvent.keyDown(input, { key: 'Escape' })

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('clicking a result row calls execute and closes the modal', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <CommandRegistryProvider>
        <TestRegistrar commands={EXTRA_COMMANDS} />
        <CommandCenter isOpen={true} onClose={onClose} />
      </CommandRegistryProvider>,
    )

    // Wait for the registration effect to commit before clicking.
    const row = await screen.findByText('Test Quick Capture')
    await user.click(row)

    const capture = EXTRA_COMMANDS.find((c) => c.id === 'test/quick-capture')!
    expect(capture.execute).toHaveBeenCalledTimes(1)
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('clicking the backdrop closes the modal', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <CommandRegistryProvider>
        <CommandCenter isOpen={true} onClose={onClose} />
      </CommandRegistryProvider>,
    )

    // The backdrop is the outer fixed-position element. We click it
    // by its data-testid to keep the test independent of layout
    // shifts.
    const backdrop = screen.getByTestId('command-center-backdrop')
    await user.click(backdrop)

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('resets the query and selection when reopened', async () => {
    const user = userEvent.setup()
    const { rerender } = render(
      <CommandRegistryProvider>
        <CommandCenter isOpen={true} onClose={vi.fn()} />
      </CommandRegistryProvider>,
    )

    const input = screen.getByPlaceholderText(/type a command/i)
    await user.type(input, 'home')

    // Close the modal.
    rerender(
      <CommandRegistryProvider>
        <CommandCenter isOpen={false} onClose={vi.fn()} />
      </CommandRegistryProvider>,
    )

    // Reopen — the input should be empty.
    rerender(
      <CommandRegistryProvider>
        <CommandCenter isOpen={true} onClose={vi.fn()} />
      </CommandRegistryProvider>,
    )

    const freshInput = screen.getByPlaceholderText(/type a command/i) as HTMLInputElement
    expect(freshInput.value).toBe('')
  })
})
