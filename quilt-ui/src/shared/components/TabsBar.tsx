import { useTabs } from '@shared/contexts/TabsContext'
import { useNavigate } from '@tanstack/react-router'
import { X, FileText, Calendar, Network, Hash, Settings as SettingsIcon, Plus } from 'lucide-react'

const TAB_ICONS: Record<string, React.ReactNode> = {
  page: <FileText size={14} />,
  journal: <Calendar size={14} />,
  graph: <Network size={14} />,
  'all-pages': <Hash size={14} />,
  settings: <SettingsIcon size={14} />,
}

export function TabsBar() {
  const { tabs, activeTabId, switchTab, closeTab, openTab } = useTabs()
  const navigate = useNavigate()

  if (tabs.length === 0) return null

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '1px',
        padding: '0 var(--space-2)',
        background: 'var(--color-surface-subtle)',
        borderBottom: '1px solid var(--color-border)',
        overflowX: 'auto',
        height: '36px',
        flexShrink: 0,
      }}
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId
        return (
          <div
            key={tab.id}
            onClick={() => {
              switchTab(tab.id)
              if (tab.type === 'page')
                navigate({ to: '/page/$name', params: { name: tab.name } })
              else if (tab.type === 'journal' && tab.params?.date)
                navigate({ to: '/journal/$date', params: { date: tab.params.date } })
              else if (tab.type === 'graph') navigate({ to: '/graph' })
              else if (tab.type === 'all-pages') navigate({ to: '/pages' })
              else if (tab.type === 'settings') navigate({ to: '/settings' })
            }}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '6px 12px',
              background: isActive ? 'var(--color-surface)' : 'transparent',
              borderBottom: isActive ? '2px solid var(--color-accent)' : '2px solid transparent',
              borderRadius: 'var(--radius-sm) var(--radius-sm) 0 0',
              cursor: 'pointer',
              fontSize: '13px',
              color: isActive ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
              minWidth: '120px',
              maxWidth: '200px',
              userSelect: 'none',
              position: 'relative',
            }}
          >
            {TAB_ICONS[tab.type]}
            <span
              style={{
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                flex: 1,
              }}
            >
              {tab.title}
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation()
                closeTab(tab.id)
              }}
              style={{
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                color: 'var(--color-text-muted)',
                padding: '0 2px',
                display: 'flex',
                alignItems: 'center',
                opacity: 0.6,
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.opacity = '1'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.opacity = '0.6'
              }}
              aria-label={`Close ${tab.title} tab`}
            >
              <X size={12} />
            </button>
          </div>
        )
      })}

      {/* New tab button — DESIGN.md §4.2: "Botón de nueva pestaña" */}
      <button
        onClick={() => {
          const name = window.prompt('Page name:')
          if (!name || !name.trim()) return
          const trimmed = name.trim()
          openTab({ name: trimmed, type: 'page', title: trimmed, params: {} })
          navigate({ to: '/page/$name', params: { name: trimmed } })
        }}
        aria-label="Open new tab"
        title="Open new tab (Ctrl+T)"
        data-testid="new-tab-button"
        style={{
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          color: 'var(--color-text-muted)',
          padding: '6px',
          borderRadius: 'var(--radius-sm)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
          marginLeft: 'var(--space-1)',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = 'var(--color-surface)'
          e.currentTarget.style.color = 'var(--color-text-primary)'
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = 'transparent'
          e.currentTarget.style.color = 'var(--color-text-muted)'
        }}
      >
        <Plus size={14} aria-hidden="true" />
      </button>
    </div>
  )
}
