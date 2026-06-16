// ─── FocusModeContext — focus mode state ──────────────────────────
//
// Provides focus mode state: whether focus mode is active and
// whether the AI panel is open. Focus mode hides the sidebar and
// other distractions, shows the editor in a centered column with
// larger fonts, and optionally shows an AI panel on the right.

import { createContext, useCallback, useContext, useState, type ReactNode } from 'react'

/** Shape of the public context. */
export interface FocusModeContextValue {
  /** True when focus mode is active. */
  isActive: boolean
  /** Toggle focus mode on/off. */
  toggle: () => void
  /** Explicitly set focus mode. */
  setActive: (active: boolean) => void
  /** True when the AI panel is open (slide-in from right). */
  isAIPanelOpen: boolean
  /** Toggle the AI panel. */
  toggleAIPanel: () => void
  /** Set AI panel open/closed. */
  setAIPanelOpen: (open: boolean) => void
}

// Safe no-op default so consumers outside a provider don't crash.
const noop = () => {}
const defaultValue: FocusModeContextValue = {
  isActive: false,
  toggle: noop,
  setActive: noop,
  isAIPanelOpen: false,
  toggleAIPanel: noop,
  setAIPanelOpen: noop,
}

const FocusModeContext = createContext<FocusModeContextValue>(defaultValue)

interface ProviderProps {
  children: ReactNode
}

/**
 * Focus mode provider. Wrap the app (or just the PageView area)
 * with this to enable focus mode state.
 */
export function FocusModeProvider({ children }: ProviderProps) {
  const [isActive, setIsActive] = useState(false)
  const [isAIPanelOpen, setIsAIPanelOpen] = useState(false)

  const toggle = useCallback(() => setIsActive(v => !v), [])
  const toggleAIPanel = useCallback(() => setIsAIPanelOpen(v => !v), [])

  const value = {
    isActive,
    toggle,
    setActive: setIsActive,
    isAIPanelOpen,
    toggleAIPanel,
    setAIPanelOpen: setIsAIPanelOpen,
  }

  return (
    <FocusModeContext.Provider value={value}>
      {children}
    </FocusModeContext.Provider>
  )
}

/**
 * Consume the focus mode context. Safe to call outside a
 * provider — returns the no-op default value instead of throwing.
 */
export function useFocusMode(): FocusModeContextValue {
  return useContext(FocusModeContext)
}
