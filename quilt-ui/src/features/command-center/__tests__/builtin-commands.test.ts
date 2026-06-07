import { describe, it, expect, vi } from 'vitest'
import { createBuiltinCommands } from '../builtin-commands'
import { CommandCategory, type Command, type CommandContext } from '../types'

/**
 * Tests for the built-in command set.
 *
 * The factory is a pure function — it does NOT call `useNavigate` or
 * `react-hot-toast` at module scope. Each test builds a fake
 * `CommandContext` and asserts that the command's `execute` body
 * reaches for the right field on that context. That keeps the test
 * surface tiny and lets us verify the wiring without rendering a
 * router or the toaster.
 */

function makeContext(overrides: Partial<CommandContext> = {}): CommandContext {
  return {
    query: undefined,
    scope: undefined,
    navigate: vi.fn() as unknown as CommandContext['navigate'],
    api: {} as CommandContext['api'],
    ...overrides,
  }
}

describe('createBuiltinCommands', () => {
  const builtins = createBuiltinCommands()

  it('returns exactly 9 built-in commands', () => {
    // The contract: 5 Navigation + 2 View + 1 Capture + 1 Help = 9.
    // This number is the source of truth for the modal's "no
    // builtins missing" smoke check; bumping it here forces a
    // conscious decision about every new builtin.
    expect(builtins).toHaveLength(9)
  })

  it('every command has the required shape (id, label, category, priority, target, execute)', () => {
    for (const cmd of builtins) {
      expect(typeof cmd.id).toBe('string')
      expect(cmd.id.length).toBeGreaterThan(0)
      expect(typeof cmd.label).toBe('string')
      expect(cmd.label.length).toBeGreaterThan(0)
      expect(Object.values(CommandCategory)).toContain(cmd.category)
      expect(typeof cmd.priority).toBe('number')
      expect(cmd.target).toBe('local')
      expect(typeof cmd.execute).toBe('function')
    }
  })

  it('all command ids are unique', () => {
    const ids = builtins.map((c) => c.id)
    const unique = new Set(ids)
    expect(unique.size).toBe(ids.length)
  })

  it('all command labels are unique', () => {
    const labels = builtins.map((c) => c.label)
    const unique = new Set(labels)
    expect(unique.size).toBe(labels.length)
  })

  it('exposes the expected category counts (5 Navigation, 2 View, 1 Capture, 1 Help)', () => {
    const byCategory = builtins.reduce<Record<string, number>>((acc, c) => {
      acc[c.category] = (acc[c.category] ?? 0) + 1
      return acc
    }, {})
    expect(byCategory[CommandCategory.Navigation]).toBe(5)
    expect(byCategory[CommandCategory.View]).toBe(2)
    expect(byCategory[CommandCategory.Capture]).toBe(1)
    expect(byCategory[CommandCategory.Help]).toBe(1)
  })

  it('uses namespaced ids (no two commands share a prefix by accident)', () => {
    // We only assert the prefix shape — `nav/`, `view/`, `capture/`,
    // `help/`. The full id list is verified in the navigation tests
    // below.
    for (const cmd of builtins) {
      const prefix = cmd.id.split('/')[0]
      expect(['nav', 'view', 'capture', 'help']).toContain(prefix)
    }
  })
})

// ──── Navigation commands ────────────────────────────────────────
//
// Every navigation command must call `ctx.navigate({ to: <route> })`
// with the right path. We exercise the `execute` function directly
// (no React render) and assert the navigate mock received the
// expected route.

describe('createBuiltinCommands — Navigation', () => {
  function find(id: string): Command {
    const cmd = createBuiltinCommands().find((c) => c.id === id)
    if (!cmd) throw new Error(`No builtin command with id ${id}`)
    return cmd
  }

  it('nav/home navigates to "/"', () => {
    const ctx = makeContext()
    find('nav/home').execute(ctx)
    expect(ctx.navigate).toHaveBeenCalledWith({ to: '/' })
  })

  it('nav/journal navigates to today\'s journal', () => {
    const ctx = makeContext()
    find('nav/journal').execute(ctx)
    const call = (ctx.navigate as unknown as ReturnType<typeof vi.fn>).mock.calls[0][0]
    expect(call.to).toBe('/journal/$date')
    // The date param must match YYYY-MM-DD — not the journalDay
    // integer form used by the legacy search modal.
    expect(call.params.date).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  it('nav/graph navigates to "/graph"', () => {
    const ctx = makeContext()
    find('nav/graph').execute(ctx)
    expect(ctx.navigate).toHaveBeenCalledWith({ to: '/graph' })
  })

  it('nav/pages navigates to "/pages"', () => {
    const ctx = makeContext()
    find('nav/pages').execute(ctx)
    expect(ctx.navigate).toHaveBeenCalledWith({ to: '/pages' })
  })

  it('nav/settings navigates to "/settings"', () => {
    const ctx = makeContext()
    find('nav/settings').execute(ctx)
    expect(ctx.navigate).toHaveBeenCalledWith({ to: '/settings' })
  })

  it('navigation commands share the priority tier (10)', () => {
    const navs = createBuiltinCommands().filter(
      (c) => c.category === CommandCategory.Navigation,
    )
    for (const c of navs) {
      expect(c.priority).toBe(10)
    }
  })
})

// ──── View commands ─────────────────────────────────────────────
//
// `view/toggle-theme` flips the data-theme attribute. We stub
// `document.documentElement.setAttribute` per-test and read the
// final value back via `getAttribute`.
//
// `view/toggle-sidebar` doesn't have a global store to flip in
// Phase 1 — it lives as React state inside the AppShell. The
// command is wired but its side-effect is a no-op stub. The
// integration test in T9 verifies the command shows up in the
// palette; the full sidebar wiring is a follow-up.

describe('createBuiltinCommands — View', () => {
  it('view/toggle-theme switches the data-theme attribute', () => {
    const cmd = createBuiltinCommands().find((c) => c.id === 'view/toggle-theme')
    expect(cmd).toBeDefined()
    if (!cmd) return

    // Start in 'light' (the default if the attribute is absent).
    document.documentElement.setAttribute('data-theme', 'light')
    cmd.execute(makeContext())
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')

    // Toggle back: dark → light.
    cmd.execute(makeContext())
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('view/toggle-theme has an execute function (sidebar toggle stub is in scope)', () => {
    const cmd = createBuiltinCommands().find((c) => c.id === 'view/toggle-sidebar')
    expect(cmd).toBeDefined()
    expect(typeof cmd?.execute).toBe('function')
    // Calling it should not throw — even though the sidebar state
    // lives in AppShell, the command is safe to invoke standalone.
    expect(() => cmd?.execute(makeContext())).not.toThrow()
  })
})

// ──── Capture command ───────────────────────────────────────────
//
// `capture/quick` opens a `window.prompt`, then calls
// `api.createBlock` with the user's input. We stub both.

describe('createBuiltinCommands — Capture', () => {
  it('capture/quick creates a block on today\'s journal when the user provides content', () => {
    const promptSpy = vi.spyOn(window, 'prompt').mockReturnValue('hello world')
    const createBlock = vi.fn().mockResolvedValue({})
    const ctx = makeContext({ api: { createBlock } as unknown as CommandContext['api'] })

    const cmd = createBuiltinCommands().find((c) => c.id === 'capture/quick')
    expect(cmd).toBeDefined()
    cmd?.execute(ctx)

    expect(promptSpy).toHaveBeenCalledTimes(1)
    expect(createBlock).toHaveBeenCalledTimes(1)
    const call = createBlock.mock.calls[0][0]
    expect(call.content).toBe('hello world')
    // pageName must be the YYYY-MM-DD string — the journal route
    // resolves a Page from the same date.
    expect(call.pageName).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  it('capture/quick does NOT call the API when the user cancels the prompt', () => {
    vi.spyOn(window, 'prompt').mockReturnValue(null)
    const createBlock = vi.fn()
    const ctx = makeContext({ api: { createBlock } as unknown as CommandContext['api'] })

    const cmd = createBuiltinCommands().find((c) => c.id === 'capture/quick')
    cmd?.execute(ctx)

    expect(createBlock).not.toHaveBeenCalled()
  })

  it('capture/quick does NOT call the API on an empty string', () => {
    vi.spyOn(window, 'prompt').mockReturnValue('   ')
    const createBlock = vi.fn()
    const ctx = makeContext({ api: { createBlock } as unknown as CommandContext['api'] })

    const cmd = createBuiltinCommands().find((c) => c.id === 'capture/quick')
    cmd?.execute(ctx)

    expect(createBlock).not.toHaveBeenCalled()
  })
})

// ──── Help command ──────────────────────────────────────────────
//
// `help/shortcuts` shows a toast summarizing the keyboard shortcuts.
// We don't import `react-hot-toast` in the assertion (the toast is a
// module-level singleton). The contract: `execute` is callable and
// does not throw.

describe('createBuiltinCommands — Help', () => {
  it('help/shortcuts is invokable without throwing', () => {
    const cmd = createBuiltinCommands().find((c) => c.id === 'help/shortcuts')
    expect(cmd).toBeDefined()
    expect(() => cmd?.execute(makeContext())).not.toThrow()
  })
})
