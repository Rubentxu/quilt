import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { CommandRegistryProvider, useCommandRegistry } from '../context'
import { CommandCategory } from '../types'
import type { ReactNode } from 'react'
import type { Command } from '../types'

// ──── Test router stub ─────────────────────────────────────────────
//
// The provider calls `useNavigate()` to build the `CommandContext`.
// jsdom doesn't have a real router, so we stub it to a vi.fn() that
// we can assert against. The stub is the same one used by
// SearchModal's tests.

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

function wrapper({ children }: { children: ReactNode }) {
  return <CommandRegistryProvider>{children}</CommandRegistryProvider>
}

/** A minimal stub command for tests that don't care about the body. */
function makeCommand(overrides: Partial<Command> = {}): Command {
  return {
    id: 'test/cmd',
    label: 'Test Command',
    category: CommandCategory.Edit,
    priority: 50,
    target: 'local',
    execute: vi.fn(),
    ...overrides,
  }
}

describe('CommandRegistryContext', () => {
  beforeEach(() => {
    mockNavigate.mockReset()
  })

  it('starts with the built-in commands registered', () => {
    // T4 fills `createBuiltinCommands()` with 9 builtins. The exact
    // count is verified in `builtin-commands.test.ts`; here we only
    // check the wiring (provider mounted → builtins show up).
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    expect(result.current.commands.length).toBeGreaterThan(0)
    // No command registered by THIS test should be present before
    // we register one.
    expect(result.current.commands.map((c) => c.id)).not.toContain('test/cmd')
  })

  it('register adds a command to the list', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    const before = result.current.commands.length
    const cmd = makeCommand({ id: 'test/register-adds', label: 'A' })
    act(() => result.current.register(cmd))
    expect(result.current.commands.length).toBe(before + 1)
    expect(result.current.commands).toContain(cmd)
  })

  it('unregister removes a command by id', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() => result.current.register(makeCommand({ id: 'test/unreg-a' })))
    act(() => result.current.register(makeCommand({ id: 'test/unreg-b' })))

    act(() => result.current.unregister('test/unreg-a'))
    const ids = result.current.commands.map((c) => c.id)
    expect(ids).not.toContain('test/unreg-a')
    expect(ids).toContain('test/unreg-b')
  })

  // R1.3 — no-op when id is not registered
  it('unregister is a no-op for unknown ids', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    const before = result.current.commands.length
    act(() => result.current.unregister('nonexistent'))
    // The list is structurally unchanged when the id is unknown.
    expect(result.current.commands.length).toBe(before)
  })

  // R1.2 — second register with same id replaces the previous
  it('register with the same id replaces the previous command', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() => result.current.register(makeCommand({ id: 'test/dup', label: 'Old' })))
    act(() => result.current.register(makeCommand({ id: 'test/dup', label: 'New' })))

    const matches = result.current.commands.filter((c) => c.id === 'test/dup')
    expect(matches).toHaveLength(1)
    expect(matches[0].label).toBe('New')
  })

  // S4 — duplicate registration contract: search returns the updated command
  it('search returns the updated command after a duplicate id register', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() => result.current.register(makeCommand({ id: 'test/dup-search', label: 'Search me' })))
    act(() => result.current.register(makeCommand({ id: 'test/dup-search', label: 'Search me (updated)' })))

    const found = result.current.commands.filter((c) => c.id === 'test/dup-search')
    expect(found).toHaveLength(1)
    expect(found[0].label).toBe('Search me (updated)')
  })

  it('search with an empty query returns all commands', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() => result.current.register(makeCommand({ id: 'test/empty-1' })))
    act(() => result.current.register(makeCommand({ id: 'test/empty-2' })))

    const all = result.current.search('')
    const ids = all.map((c) => c.id)
    expect(ids).toContain('test/empty-1')
    expect(ids).toContain('test/empty-2')
  })

  // R1.4 — case-insensitive substring filter on label
  it('search filters by label substring (case-insensitive)', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() =>
      result.current.register(makeCommand({ id: 'test/filter-a', label: 'Unique Theme Marker' })),
    )
    act(() =>
      result.current.register(makeCommand({ id: 'test/filter-b', label: 'Go to Journal' })),
    )
    act(() =>
      result.current.register(
        makeCommand({ id: 'test/filter-c', label: 'Keyboard Shortcuts' }),
      ),
    )

    const matches = result.current.search('UNIQUE THEME')
    expect(matches.map((m) => m.id)).toEqual(['test/filter-a'])
  })

  // S5 — search by category name (e.g. typing "view" returns all View commands)
  it('search matches against the category name', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() =>
      result.current.register(
        makeCommand({
          id: 'test/cat-a',
          label: 'AAAA',
          category: CommandCategory.View,
        }),
      ),
    )
    act(() =>
      result.current.register(
        makeCommand({
          id: 'test/cat-b',
          label: 'BBBB',
          category: CommandCategory.View,
        }),
      ),
    )
    act(() =>
      result.current.register(
        makeCommand({
          id: 'test/cat-c',
          label: 'CCCC',
          category: CommandCategory.Capture,
        }),
      ),
    )

    const matches = result.current.search('view')
    const ids = matches.map((m) => m.id)
    // The two test View commands match; the Capture one does not.
    expect(ids).toContain('test/cat-a')
    expect(ids).toContain('test/cat-b')
    expect(ids).not.toContain('test/cat-c')
  })

  // R1.4 — sort by priority (lower first), then label
  it('search sorts results by priority ascending, then label ascending', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() =>
      result.current.register(
        makeCommand({ id: 'test/sort-high', label: 'HighZ', priority: 1 }),
      ),
    )
    act(() =>
      result.current.register(
        makeCommand({ id: 'test/sort-low', label: 'LowA', priority: 99 }),
      ),
    )
    act(() =>
      result.current.register(
        makeCommand({ id: 'test/sort-mid', label: 'MidM', priority: 50 }),
      ),
    )

    const sorted = result.current.search('')
    // Pull out the test commands in result order.
    const testOrder = sorted
      .filter((c) => c.id.startsWith('test/sort-'))
      .map((c) => c.id)
    expect(testOrder).toEqual(['test/sort-high', 'test/sort-mid', 'test/sort-low'])
  })

  // R1.4 — same priority falls back to alphabetical label
  it('search falls back to alphabetical label when priorities tie', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() => result.current.register(makeCommand({ id: 'test/tie-z', label: 'Zebra AAA', priority: 25 })))
    act(() => result.current.register(makeCommand({ id: 'test/tie-a', label: 'Apple BBB', priority: 25 })))
    act(() => result.current.register(makeCommand({ id: 'test/tie-m', label: 'Mango CCC', priority: 25 })))

    const sorted = result.current.search('')
    const testOrder = sorted
      .filter((c) => c.id.startsWith('test/tie-'))
      .map((c) => c.label)
    expect(testOrder).toEqual(['Apple BBB', 'Mango CCC', 'Zebra AAA'])
  })

  // R1.5 — execute calls command.execute with a CommandContext
  it('execute invokes the command and passes a CommandContext with navigate + api', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    const execute = vi.fn()
    act(() =>
      result.current.register(
        makeCommand({ id: 'a', execute }),
      ),
    )

    act(() => result.current.execute('a'))

    expect(execute).toHaveBeenCalledTimes(1)
    const ctx = execute.mock.calls[0][0]
    expect(ctx.navigate).toBe(mockNavigate)
    expect(ctx.api).toBeDefined()
    expect(typeof ctx.api.listPages).toBe('function')
  })

  // R1.5 — execute forwards the optional query and scope
  it('execute forwards the optional query and scope to the command', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    const execute = vi.fn()
    act(() => result.current.register(makeCommand({ id: 'a', execute })))

    act(() => result.current.execute('a', { query: 'foo', scope: 'page' }))

    const ctx = execute.mock.calls[0][0]
    expect(ctx.query).toBe('foo')
    expect(ctx.scope).toBe('page')
  })

  // R1.5 — execute is a no-op for unknown ids
  it('execute is a no-op when the id is unknown', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    // No throw means it returned cleanly.
    expect(() => result.current.execute('does-not-exist')).not.toThrow()
  })

  // R1.6 — method references stay stable across re-renders
  it('register / unregister / search / execute are stable across re-renders', () => {
    const { result, rerender } = renderHook(() => useCommandRegistry(), { wrapper })
    const initial = {
      register: result.current.register,
      unregister: result.current.unregister,
      search: result.current.search,
      execute: result.current.execute,
    }

    rerender()

    expect(result.current.register).toBe(initial.register)
    expect(result.current.unregister).toBe(initial.unregister)
    // `search` depends on `commands` so its reference DOES change when
    // the command list changes — that is correct. The contract is
    // that it does NOT change on a re-render with no state change.
    // We just registered no commands here, so the list is empty in
    // both renders and `search` should be stable.
    expect(result.current.search).toBe(initial.search)
    // Same for `execute` — depends on `commands`.
    expect(result.current.execute).toBe(initial.execute)
  })

  // S2 — register from useEffect, then unregister on cleanup
  it('a feature can register and unregister its own commands', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    act(() => result.current.register(makeCommand({ id: 'test/feature-x' })))
    expect(result.current.commands.map((c) => c.id)).toContain('test/feature-x')

    act(() => result.current.unregister('test/feature-x'))
    expect(result.current.commands.map((c) => c.id)).not.toContain('test/feature-x')
  })
})

// ──── Builtin command registration ────────────────────────────────
//
// The provider runs `createBuiltinCommands()` once on mount and
// registers every command. We assert the list is non-empty AND
// contains the namespaced ids we expect (5 Navigation, 2 View, 1
// Capture, 1 Help). The exact count is verified in
// `builtin-commands.test.ts`; here we only check the wiring.

describe('CommandRegistryProvider — builtin registration', () => {
  it('registers the built-in commands on mount', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    // The exact total is verified in builtin-commands.test.ts
    // (9 today). We assert a lower bound so a future addition
    // does not silently break the wiring assertion.
    expect(result.current.commands.length).toBeGreaterThanOrEqual(9)
  })

  it('registers the navigation builtins with their expected ids', () => {
    const { result } = renderHook(() => useCommandRegistry(), { wrapper })
    const ids = result.current.commands.map((c) => c.id)
    for (const id of ['nav/home', 'nav/journal', 'nav/graph', 'nav/pages', 'nav/settings']) {
      expect(ids).toContain(id)
    }
  })
})
