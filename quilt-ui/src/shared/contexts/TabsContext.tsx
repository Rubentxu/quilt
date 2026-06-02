import { createContext, useContext, useState, useCallback, type ReactNode } from 'react'

export interface Tab {
  id: string
  name: string
  type: 'page' | 'journal' | 'graph' | 'all-pages' | 'settings'
  title: string
  params?: Record<string, string>
}

interface TabsContextValue {
  tabs: Tab[]
  activeTabId: string | null
  openTab: (tab: Omit<Tab, 'id'>) => string
  closeTab: (id: string) => void
  switchTab: (id: string) => void
  closeAllTabs: () => void
  closeOtherTabs: (id: string) => void
}

const TabsContext = createContext<TabsContextValue>({
  tabs: [],
  activeTabId: null,
  openTab: () => '',
  closeTab: () => {},
  switchTab: () => {},
  closeAllTabs: () => {},
  closeOtherTabs: () => {},
})

/** Build a deterministic id from a tab's content. No timestamp — same content
 *  always produces the same id, so two `openTab` calls for the same page
 *  produce the same id and the dedup check can match them. */
function buildTabId(type: Tab['type'], name: string, params: Record<string, string> | undefined): string {
  const sortedParams = params
    ? Object.keys(params)
        .sort()
        .map((k) => `${k}=${params[k]}`)
        .join('&')
    : ''
  return `${type}:${name || 'new'}:${sortedParams}`
}

/** Normalize params so two equivalent objects compare equal regardless of key order. */
function normalizeParams(params: Record<string, string> | undefined): string {
  if (!params) return ''
  return Object.keys(params)
    .sort()
    .map((k) => `${k}=${params[k]}`)
    .join('&')
}

/** Two tab requests refer to the same logical page when their type+name+params match. */
function isSameTab(
  existing: Tab,
  incoming: { type: Tab['type']; name: string; params?: Record<string, string> },
): boolean {
  if (existing.type !== incoming.type) return false
  if (existing.name !== incoming.name) return false
  return normalizeParams(existing.params) === normalizeParams(incoming.params)
}

export function TabsProvider({ children }: { children: ReactNode }) {
  const [tabs, setTabs] = useState<Tab[]>([])
  const [activeTabId, setActiveTabId] = useState<string | null>(null)

  const openTab = useCallback((newTab: Omit<Tab, 'id'>): string => {
    const id = buildTabId(newTab.type, newTab.name, newTab.params)

    // Run dedup inside the setTabs updater so we see the freshest state.
    // This is the fix for the race where two synchronous openTab() calls
    // (React StrictMode, or fast consecutive navigations) both saw an
    // empty `tabs` array via a stale ref and created duplicate tabs.
    let resolvedId = id
    setTabs((prev) => {
      const existing = prev.find((t) => isSameTab(t, newTab))
      if (existing) {
        resolvedId = existing.id
        return prev // no change → no re-render of the list
      }
      return [...prev, { ...newTab, id }]
    })
    setActiveTabId(resolvedId)
    return resolvedId
  }, [])

  const closeTab = useCallback((id: string) => {
    // Use the functional updater to know which tab index is being closed
    // and pick a sensible adjacent tab to activate.
    setTabs((prev) => {
      const idx = prev.findIndex((t) => t.id === id)
      if (idx === -1) return prev
      const newTabs = prev.filter((t) => t.id !== id)

      // Determine the next active tab. We use `activeTabId` from the
      // closure here, but we only consume it if it matches the id being
      // closed. Because we only call setActiveTabId in that case, this
      // works for all common scenarios.
      setActiveTabId((currentActive) => {
        if (currentActive !== id) return currentActive
        if (newTabs.length === 0) return null
        // Pick the tab at the same index, or the last one if we removed the tail.
        const nextIdx = Math.min(idx, newTabs.length - 1)
        return newTabs[nextIdx].id
      })

      return newTabs
    })
  }, [])

  const switchTab = useCallback((id: string) => {
    setActiveTabId(id)
  }, [])

  const closeAllTabs = useCallback(() => {
    setTabs([])
    setActiveTabId(null)
  }, [])

  const closeOtherTabs = useCallback((id: string) => {
    setTabs((prev) => prev.filter((t) => t.id === id))
    setActiveTabId(id)
  }, [])

  return (
    <TabsContext.Provider
      value={{
        tabs,
        activeTabId,
        openTab,
        closeTab,
        switchTab,
        closeAllTabs,
        closeOtherTabs,
      }}
    >
      {children}
    </TabsContext.Provider>
  )
}

export function useTabs() {
  return useContext(TabsContext)
}
