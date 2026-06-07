import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import { useNavigate } from '@tanstack/react-router'
import {
  CommandCategory,
  DEFAULT_COMMAND_PRIORITY,
  type Command,
  type CommandContext,
} from './types'
import { api } from '@core/api-client'
import { createBuiltinCommands } from './builtin-commands'

interface CommandRegistryContextValue {
  /** Read-only list of all currently-registered commands. */
  commands: readonly Command[]
  /**
   * Register a command. If a command with the same `id` is already
   * registered, the new one REPLACES the old (R1.2 — dedup by id).
   * Returns nothing; the registry does not surface registration
   * failures (it has none).
   */
  register: (command: Command) => void
  /**
   * Unregister a command by id. No-op if the id is not currently
   * registered (R1.3).
   */
  unregister: (id: string) => void
  /**
   * Filter and rank commands against a query. Case-insensitive
   * substring match against `label` and against the category's
   * display name. Empty query returns all commands.
   *
   * Sort order: `priority` ASC, then `label` ASC. Lower priority
   * numbers surface first so Navigation (10) beats View (20)
   * beats Capture (30) beats Help (40).
   */
  search: (query: string) => Command[]
  /**
   * Find a command by id and invoke its `execute` with a fully-built
   * `CommandContext`. No-op when the id is unknown — callers don't
   * have to guard against stale ids in `onUnmounted` paths.
   */
  execute: (id: string, ctx?: Partial<CommandContext>) => void
}

const CommandRegistryContext = createContext<CommandRegistryContextValue>({
  commands: [],
  register: () => {},
  unregister: () => {},
  search: () => [],
  execute: () => {},
})

interface CommandRegistryProviderProps {
  children: ReactNode
}

export function CommandRegistryProvider({ children }: CommandRegistryProviderProps) {
  const [commands, setCommands] = useState<Command[]>([])
  const navigate = useNavigate()
  // Keep a ref to the current navigate so `execute` always invokes
  // the latest closure without re-binding every render. `useNavigate`
  // is itself stable in TanStack Router, but the ref pattern matches
  // TabsContext and insulates us from any future router refactor.
  const navigateRef = useRef(navigate)
  navigateRef.current = navigate

  // ── register ─────────────────────────────────────────────────────
  //
  // Functional updater so two synchronous `register()` calls in the
  // same render (StrictMode double-invoke, or a feature's useEffect
  // mounting in the same tick as a builtin registration) cannot
  // stomp on each other. We dedup by id INSIDE the updater — the
  // command list is the only source of truth, never a stale ref.
  const register = useCallback((command: Command) => {
    setCommands((prev) => {
      const idx = prev.findIndex((c) => c.id === command.id)
      if (idx === -1) return [...prev, command]
      const next = prev.slice()
      next[idx] = command
      return next
    })
  }, [])

  // ── unregister ───────────────────────────────────────────────────
  // Filter inside the updater; if the id is not present, the
  // returned array is structurally equal to `prev` and React bails
  // out of the re-render.
  const unregister = useCallback((id: string) => {
    setCommands((prev) => {
      if (!prev.some((c) => c.id === id)) return prev
      return prev.filter((c) => c.id !== id)
    })
  }, [])

  // ── search ───────────────────────────────────────────────────────
  // `useCallback` keeps the function reference stable across renders
  // — important because modal/list code that uses `search` as a
  // useEffect dep should not refire on every keystroke.
  const search = useCallback(
    (query: string): Command[] => {
      const q = query.trim().toLowerCase()
      const filtered = q === ''
        ? commands
        : commands.filter((c) => {
            const label = c.label.toLowerCase()
            const category = c.category.toLowerCase()
            return label.includes(q) || category.includes(q)
          })
      return [...filtered].sort((a, b) => {
        if (a.priority !== b.priority) return a.priority - b.priority
        return a.label.localeCompare(b.label)
      })
    },
    [commands],
  )

  // ── execute ──────────────────────────────────────────────────────
  // Builds a CommandContext on the fly. The query / scope fields
  // come from the optional `ctx` argument so the modal can pass
  // "what the user typed" without each command having to read it
  // from a global.
  const execute = useCallback(
    (id: string, ctx?: Partial<CommandContext>) => {
      const cmd = commands.find((c) => c.id === id)
      if (!cmd) return
      const fullCtx: CommandContext = {
        query: ctx?.query,
        scope: ctx?.scope,
        navigate: ctx?.navigate ?? navigateRef.current,
        api: ctx?.api ?? api,
      }
      cmd.execute(fullCtx)
    },
    [commands],
  )

  // ── builtin registration ─────────────────────────────────────────
  //
  // Built-in commands are registered ONCE on mount and torn down on
  // unmount. We use `useEffect` (not `useMemo`) because we need a
  // cleanup return — the cleanup runs on unmount AND on StrictMode's
  // second mount, so the list never leaks duplicate builtins.
  //
  // The ref guards against the StrictMode double-invoke pattern:
  // the first effect runs register → cleanup → register, but the
  // second register is on a fresh provider so the list is already
  // empty. Either way, we end up with exactly the builtin set.
  useEffect(() => {
    const builtins = createBuiltinCommands()
    for (const cmd of builtins) {
      register(cmd)
    }
    return () => {
      for (const cmd of builtins) {
        unregister(cmd.id)
      }
    }
    // The registration happens once for the provider's lifetime.
    // `register` and `unregister` are stable useCallbacks, so this
    // effect's deps are effectively constant.
  }, [register, unregister])

  // Memoize the context value so consumers can use the value as a
  // dep without churning every render. Methods are stable
  // (useCallback), so this is essentially a no-op cost.
  const value = useMemo<CommandRegistryContextValue>(
    () => ({ commands, register, unregister, search, execute }),
    [commands, register, unregister, search, execute],
  )

  return (
    <CommandRegistryContext.Provider value={value}>
      {children}
    </CommandRegistryContext.Provider>
  )
}

export function useCommandRegistry(): CommandRegistryContextValue {
  return useContext(CommandRegistryContext)
}

// Re-export the category enum + default priority for callers that
// import everything from the context module.
export { CommandCategory, DEFAULT_COMMAND_PRIORITY }
