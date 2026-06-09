// ─── PanelVisibilityContext — dashboard panel state ──────────────
//
// Single source of truth for "which panels does the user want to
// see right now". Lives at the top of the React tree (just below
// the router), persists to localStorage, and exposes a small API
// for the LayoutMenu, the AppShell, and the CommandRegistry to
// read or mutate.
//
// Design notes (see ADR-DRAFT `dashboard-layout-no-work-modes.md`):
//   - This is FRONTEND configuration only. There is no
//     `DashboardLayout` entity in the Rust domain.
//   - A "preset" is just a named `Set<PanelId>`. There is no
//     "WorkMode" concept (rejected by the auto-grill session).
//   - Persistence is best-effort: if localStorage is unavailable
//     (private mode / quota), the in-memory state still works for
//     the current session.

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
import {
  getPreset,
  PRESETS,
  type PanelId,
  type PresetId,
} from './presets'

/**
 * Canonical human-readable label for every known panel. This is the
 * single source of truth — `presets.ts` re-exports this constant
 * (it does not redefine it) so consumers can import from either
 * module without risk of drift.
 */
export const PANEL_LABELS: Record<PanelId, string> = {
  sidebar: 'Sidebar',
  backlinks: 'Backlinks',
  'agent-activity': 'Agent activity',
  outline: 'Outline',
  'structural-graph': 'Structural graph',
  'semantic-insight': 'Semantic insight',
}

/** localStorage key for the dashboard layout. */
export const DASHBOARD_STORAGE_KEY = 'quilt-dashboard-layout'

/**
 * Custom event the CommandRegistry (and any non-React caller)
 * dispatches to mutate the panel set. The provider listens and
 * applies the requested change. This is the same "custom event"
 * pattern the legacy `view/toggle-sidebar` command comment
 * documents — it lets us keep the registry factory free of React
 * hooks.
 */
export const DASHBOARD_EVENT = 'quilt:dashboard-layout-change'

/** Detail shapes the provider understands. */
export type DashboardEventDetail =
  | { type: 'set'; panels: PanelId[] }
  | { type: 'toggle'; panel: PanelId }
  | { type: 'preset'; preset: PresetId }

/**
 * Dispatch a dashboard layout change. Safe to call from anywhere
 * (the CommandRegistry's execute path, browser devtools, tests).
 * No-op when the host environment lacks `window`.
 */
export function dispatchDashboardChange(detail: DashboardEventDetail): void {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent<DashboardEventDetail>(DASHBOARD_EVENT, { detail }))
}

/**
 * Re-export of the panel-id list as a stable, ordered array. We
 * keep this here (and not in `presets.ts`) because it's part of
 * the public context API — `usePanelVisibility().panels` would
 * need it too if we add that later.
 */
export const DEFAULT_PANELS: readonly PanelId[] = [
  'sidebar',
  'backlinks',
  'agent-activity',
  'outline',
  'structural-graph',
  'semantic-insight',
]

/** Shape of the public context. */
export interface PanelVisibilityContextValue {
  /** Which panels are currently visible. */
  visiblePanels: Set<PanelId>
  /** Replace the visibility set wholesale. */
  setVisiblePanels: (next: Set<PanelId>) => void
  /** Flip a single panel id. */
  togglePanel: (id: PanelId) => void
  /** Apply one of the named presets. Unknown ids are no-ops. */
  applyPreset: (id: PresetId) => void
  /**
   * The id of the most recently applied preset, or `null` when
   * the user has been editing individual checkboxes and the
   * current set no longer matches any preset.
   */
  lastAppliedPreset: PresetId | null
  /** True once the provider has finished its initial localStorage read. */
  isHydrated: boolean
}

// Safe no-op default so consumers outside a provider don't crash.
const noop = () => {}
const EMPTY_SET: Set<PanelId> = new Set()
const defaultValue: PanelVisibilityContextValue = {
  visiblePanels: EMPTY_SET,
  setVisiblePanels: noop,
  togglePanel: noop,
  applyPreset: noop,
  lastAppliedPreset: null,
  isHydrated: false,
}

const PanelVisibilityContext = createContext<PanelVisibilityContextValue>(defaultValue)

/** Safe localStorage read. Returns `null` on any error. */
function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    // Private mode / disabled storage — degrade gracefully.
    return null
  }
}

/** Safe localStorage write. Swallows any error. */
function writeStorage(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    // Quota / private mode — the in-memory state is still correct
    // for this session; we just can't persist.
  }
}

/**
 * Read a `Set<PanelId>` back from localStorage. Returns `null`
 * when the stored value is missing or unparseable.
 */
function parseStoredSet(raw: string | null): Set<PanelId> | null {
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return null
    const valid: PanelId[] = []
    for (const item of parsed) {
      if (typeof item === 'string' && (DEFAULT_PANELS as readonly string[]).includes(item)) {
        valid.push(item as PanelId)
      }
    }
    return new Set(valid)
  } catch {
    return null
  }
}

interface ProviderProps {
  children: ReactNode
}

export function PanelVisibilityProvider({ children }: ProviderProps) {
  // Initial state comes from the default preset. The effect below
  // swaps in the localStorage value once the component has mounted
  // (we don't read localStorage during render — that would be a
  // server/client hydration mismatch in a real SSR setup, and
  // it's also flagged by the linter).
  const [visiblePanels, setVisiblePanelsState] = useState<Set<PanelId>>(
    () => getPreset('default'),
  )
  const [isHydrated, setIsHydrated] = useState(false)
  // Keep the latest panels in a ref so the persistence effect can
  // write on every change without re-binding.
  const panelsRef = useRef(visiblePanels)
  panelsRef.current = visiblePanels

  // ── Hydrate from localStorage on mount ──────────────────────────
  useEffect(() => {
    const stored = parseStoredSet(readStorage(DASHBOARD_STORAGE_KEY))
    if (stored && stored.size > 0) {
      setVisiblePanelsState(stored)
    }
    setIsHydrated(true)
  }, [])

  // ── Persist on change (after hydration) ─────────────────────────
  useEffect(() => {
    if (!isHydrated) return
    writeStorage(
      DASHBOARD_STORAGE_KEY,
      JSON.stringify(Array.from(panelsRef.current)),
    )
  }, [visiblePanels, isHydrated])

  // ── Listen for non-React callers (CommandRegistry) ─────────────
  //
  // The CommandRegistry's `execute` runs outside of React. To let
  // commands like `layout/toggle-sidebar` or `layout/switch-to-focus`
  // mutate the panel set, we expose a custom DOM event the
  // provider listens to. The handler mirrors the public context
  // API (set / toggle / preset) so callers don't need to reach
  // into React internals.
  useEffect(() => {
    function handle(event: Event) {
      const detail = (event as CustomEvent<DashboardEventDetail>).detail
      if (!detail || typeof detail !== 'object') return
      if (detail.type === 'set') {
        setVisiblePanelsState(new Set(detail.panels))
      } else if (detail.type === 'toggle') {
        setVisiblePanelsState((prev) => {
          const next = new Set(prev)
          if (next.has(detail.panel)) next.delete(detail.panel)
          else next.add(detail.panel)
          return next
        })
      } else if (detail.type === 'preset') {
        if (PRESETS[detail.preset]) {
          setVisiblePanelsState(getPreset(detail.preset))
        }
      }
    }
    window.addEventListener(DASHBOARD_EVENT, handle)
    return () => window.removeEventListener(DASHBOARD_EVENT, handle)
  }, [])

  // ── Public API ─────────────────────────────────────────────────
  const setVisiblePanels = useCallback((next: Set<PanelId>) => {
    setVisiblePanelsState(new Set(next))
  }, [])

  const togglePanel = useCallback((id: PanelId) => {
    setVisiblePanelsState((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }, [])

  const applyPreset = useCallback((id: PresetId) => {
    if (!PRESETS[id]) return
    setVisiblePanelsState(getPreset(id))
  }, [])

  // Derive the "last applied preset" so the LayoutMenu can show
  // which preset the current set most closely matches.
  const lastAppliedPreset = useMemo<PresetId | null>(() => {
    for (const id of Object.keys(PRESETS) as PresetId[]) {
      const preset = PRESETS[id]
      if (preset.size !== visiblePanels.size) continue
      let same = true
      for (const panel of preset) {
        if (!visiblePanels.has(panel)) {
          same = false
          break
        }
      }
      if (same) return id
    }
    return null
  }, [visiblePanels])

  const value = useMemo<PanelVisibilityContextValue>(
    () => ({
      visiblePanels,
      setVisiblePanels,
      togglePanel,
      applyPreset,
      lastAppliedPreset,
      isHydrated,
    }),
    [
      visiblePanels,
      setVisiblePanels,
      togglePanel,
      applyPreset,
      lastAppliedPreset,
      isHydrated,
    ],
  )

  return (
    <PanelVisibilityContext.Provider value={value}>
      {children}
    </PanelVisibilityContext.Provider>
  )
}

/**
 * Consume the panel visibility context. Safe to call outside a
 * provider — returns the no-op default value instead of throwing.
 */
export function usePanelVisibility(): PanelVisibilityContextValue {
  return useContext(PanelVisibilityContext)
}
