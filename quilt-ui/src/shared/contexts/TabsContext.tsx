import { createContext, useContext, useState, useCallback, useRef, useEffect, type ReactNode } from 'react'

export interface Tab {
  id: string // unique id (page name + timestamp)
  name: string // page name
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

export function TabsProvider({ children }: { children: ReactNode }) {
  const [tabs, setTabs] = useState<Tab[]>([])
  const [activeTabId, setActiveTabId] = useState<string | null>(null)
  // Keep a ref for non-stale reads inside callbacks
  const tabsRef = useRef(tabs)
  useEffect(() => {
    tabsRef.current = tabs
  }, [tabs])

  const openTab = useCallback((newTab: Omit<Tab, 'id'>) => {
    const id = `${newTab.type}:${newTab.name || 'new'}:${newTab.params?.date || ''}:${Date.now()}`

    // Check if same page is already open (using ref to avoid stale closure)
    const existing = tabsRef.current.find((t) => {
      if (t.type !== newTab.type) return false
      if (t.name !== newTab.name) return false
      return JSON.stringify(t.params) === JSON.stringify(newTab.params)
    })

    if (existing) {
      setActiveTabId(existing.id)
      return existing.id
    }

    setTabs((prev) => [...prev, { ...newTab, id }])
    setActiveTabId(id)
    return id
  }, [])

  const closeTab = useCallback(
    (id: string) => {
      setTabs((prev) => {
        const idx = prev.findIndex((t) => t.id === id)
        if (idx === -1) return prev
        const newTabs = prev.filter((t) => t.id !== id)

        // If closing active tab, activate adjacent
        if (activeTabId === id) {
          const newActive = newTabs[Math.min(idx, newTabs.length - 1)]
          // Schedule active tab update (can't setState inside setTabs in some React versions)
          queueMicrotask(() => setActiveTabId(newActive?.id || null))
        }

        return newTabs
      })
    },
    [activeTabId],
  )

  const switchTab = useCallback((id: string) => {
    setActiveTabId(id)
  }, [])

  const closeAllTabs = useCallback(() => {
    setTabs([])
    setActiveTabId(null)
  }, [])

  const closeOtherTabs = useCallback(
    (id: string) => {
      setTabs((prev) => prev.filter((t) => t.id === id))
      setActiveTabId(id)
    },
    [],
  )

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
