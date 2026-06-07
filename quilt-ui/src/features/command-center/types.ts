import { useNavigate } from '@tanstack/react-router'
import type { api } from '@core/api-client'

/**
 * The `navigate` argument passed to every command's `execute`.
 *
 * We type it via `ReturnType<typeof useNavigate>` (rather than the
 * `NavigateFunction` type from `@tanstack/react-router`, which is not
 * a public export) — this is the same idiom used by
 * `AppShell.executeGoTo`. The type is structural; the runtime value
 * is whatever `useNavigate()` returns inside the current Router
 * scope.
 */
export type CommandNavigate = ReturnType<typeof useNavigate>

/**
 * Categories for the CommandRegistry. Each command belongs to exactly one
 * category; the modal surfaces the category as a colored badge so users can
 * scan results by intent (Navigation vs View vs Capture vs Edit vs Help).
 */
export enum CommandCategory {
  Navigation = 'Navigation',
  View = 'View',
  Capture = 'Capture',
  Edit = 'Edit',
  Help = 'Help',
}

/**
 * A single executable action in the CommandRegistry.
 *
 * - `id` must be unique — second `register()` with the same id replaces the
 *   previous one. Use a namespaced id (`nav/home`, `capture/quick`, ...) so
 *   feature-local commands don't collide with built-ins.
 * - `priority` — lower = shown first. Defaults to 50 when callers don't
 *   specify one. Built-in navigation commands use 10, View uses 20,
 *   Capture 30, Help 40.
 * - `target` — Phase 1 only honors `'local'`. The `'mcp'` value is
 *   reserved for Phase 2 dispatch (see ADR draft).
 * - `execute` is called with a `CommandContext` that the registry builds
 *   from the current `useNavigate()` + the `api` client.
 */
export interface Command {
  id: string
  label: string
  category: CommandCategory
  shortcut?: string
  priority: number
  target: 'local' | 'mcp'
  execute: (ctx: CommandContext) => void
}

/**
 * Runtime context passed to every command's `execute`. The registry
 * builds this from the current React tree (navigate) and the static
 * `api` client; commands should never reach for globals when they
 * have access to this.
 *
 * `query` is the user's current search string when the command is
 * invoked from the modal — useful for "Quick Capture" and similar
 * context-aware actions. `scope` is reserved for a future filter
 * mode (e.g. "page-scoped commands" when invoked from inside
 * PageView).
 */
export interface CommandContext {
  query?: string
  scope?: string
  navigate: CommandNavigate
  api: typeof api
}

/** Default priority for commands that don't declare one. Lower = first. */
export const DEFAULT_COMMAND_PRIORITY = 50
