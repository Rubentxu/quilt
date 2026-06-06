import { useState, useEffect, type KeyboardEvent } from 'react'
import { Link, useNavigate, useLocation } from '@tanstack/react-router'
import { Search, Calendar, FileText, Plus, Clock, LayoutList, Network, X, Bot, Star, ChevronDown } from 'lucide-react'
import { Table } from 'lucide-react'
import toast from 'react-hot-toast'
import { api } from '@core/api-client'
import type { Page } from '@shared/types/api'
import { AgentActivityPanel } from '@features/cognitive/AgentActivityPanel'


function readFavorites(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.FAVORITES)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === 'string') : []
  } catch {
    return []
  }
}

interface SidebarProps {
  collapsed: boolean
  onOpenSearch?: () => void
  onClose?: () => void
}

function formatToday(): { url: string; label: string } {
  const now = new Date()
  const y = now.getFullYear()
  const m = String(now.getMonth() + 1).padStart(2, '0')
  const d = String(now.getDate()).padStart(2, '0')
  const url = `${y}-${m}-${d}`
  const label = now.toLocaleDateString('en-US', {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  })
  return { url, label }
}


import { GroupHeader } from './sections/GroupHeader'
import { SidebarItem } from './sections/SidebarItem'
import { SidebarSkeleton } from './sections/SidebarSkeleton'
import { STORAGE_KEYS } from './storage-keys'
import { RecentsSection } from './sections/RecentsSection'
import { TemplateSection } from './sections/TemplateSection'
import { FAVORITES_CHANGED_EVENT } from '@features/favorites/favoritesStore'
export function Sidebar({ collapsed, onOpenSearch, onClose }: SidebarProps) {
  const [pages, setPages] = useState<Page[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [searchFocused, setSearchFocused] = useState(false)
  const [showAgentActivity, setShowAgentActivity] = useState(false)
  const [favorites, setFavorites] = useState<string[]>(() => readFavorites())
  const navigate = useNavigate()
  const location = useLocation()
  const today = formatToday()

  function toggleFavorite(name: string) {
    setFavorites((prev) => {
      const next = prev.includes(name)
        ? prev.filter((n) => n !== name)
        : [...prev, name]
      localStorage.setItem(STORAGE_KEYS.FAVORITES, JSON.stringify(next))
      return next
    })
  }

  const favoritePages = pages.filter(
    (p) => !p.journal && favorites.includes(p.name)
  )

  useEffect(() => {
    let cancelled = false
    api.listPages()
      .then((data) => {
        if (!cancelled) {
          setPages(data)
          setLoading(false)
        }
      })
      .catch((err) => {
        if (!cancelled) {
          toast.error(`Failed to load pages: ${err instanceof Error ? err.message : 'Unknown error'}`)
          setLoading(false)
        }
      })
    return () => { cancelled = true }
  }, [])

  // F2 of quilt-fase2-ux-dead-buttons — when the page header's
  // star button toggles a favorite (or another tab does), re-read
  // from localStorage so the Sidebar's Favorites section stays in
  // sync without a full reload.
  useEffect(() => {
    function onFavoritesChanged() {
      setFavorites(readFavorites())
    }
    window.addEventListener(FAVORITES_CHANGED_EVENT, onFavoritesChanged)
    return () => window.removeEventListener(FAVORITES_CHANGED_EVENT, onFavoritesChanged)
  }, [])

  const regularPages = pages.filter((p) => !p.journal)

  async function handleNewPage() {
    const name = window.prompt('Page name:')
    if (!name || !name.trim()) return

    setCreating(true)
    try {
      const page = await api.createPage({ name: name.trim().toLowerCase() })
      setPages((prev) => [...prev, page])
      navigate({ to: `/page/${encodeURIComponent(page.name)}` })
    } catch (err) {
      toast.error(`Failed to create page: ${err instanceof Error ? err.message : 'Unknown error'}`)
    } finally {
      setCreating(false)
    }
  }

  function handleSearchKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault()
      onOpenSearch?.()
    }
  }

  return (
    <div
      data-testid="sidebar"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
      }}
    >
      {/* Close button for mobile drawer */}
      {onClose && (
        <div
          style={{
            display: 'flex',
            justifyContent: 'flex-end',
            padding: 'var(--space-1) var(--space-1) 0',
          }}
        >
          <button
            onClick={onClose}
            aria-label="Close sidebar"
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-secondary)',
              padding: 'var(--space-1)',
              borderRadius: 'var(--radius-md)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
            className="topbar-action"
          >
            <X size={18} />
          </button>
        </div>
      )}

      {/* Workspace selector */}
      {!collapsed && (
        <div
          style={{
            padding: 'var(--space-4) var(--space-4) var(--space-3)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: 'var(--space-2)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', minWidth: 0 }}>
            <div
              style={{
                width: '28px',
                height: '28px',
                borderRadius: '9px',
                background: 'linear-gradient(180deg, #4F7BFF 0%, #355CFF 100%)',
                color: '#fff',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: '14px',
                fontWeight: 700,
                boxShadow: '0 6px 18px rgba(79, 123, 255, 0.18)',
              }}
            >
              Q
            </div>
            <div
              style={{
                minWidth: 0,
                fontSize: '14px',
                fontWeight: 700,
                color: 'var(--color-text-primary)',
                letterSpacing: '-0.01em',
                whiteSpace: 'nowrap',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
              }}
            >
              Quilt Workspace
            </div>
          </div>
          <button
            type="button"
            aria-label="Workspace options"
            className="ghost-icon-button"
            style={{
              width: '28px',
              height: '28px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
            }}
          >
            <ChevronDown size={15} />
          </button>
        </div>
      )}

      {/* Search input */}
      {!collapsed && (
        <div
          style={{
            padding: '0 var(--space-3) var(--space-3)',
            position: 'relative',
          }}
        >
          <div
            className="surface-input"
            style={{
              display: 'flex',
              alignItems: 'center',
              background: searchFocused ? 'var(--color-surface)' : 'var(--color-surface-subtle)',
              border: searchFocused ? '1px solid rgba(37, 99, 235, 0.18)' : '1px solid transparent',
              borderRadius: '12px',
              padding: '0 10px',
              height: '40px',
              boxShadow: searchFocused ? '0 0 0 3px rgba(37, 99, 235, 0.08)' : 'none',
              transition: 'border var(--motion-fast) var(--ease-standard), background var(--motion-fast) var(--ease-standard), box-shadow var(--motion-fast) var(--ease-standard)',
              cursor: 'text',
            }}
            onClick={onOpenSearch}
          >
            <Search size={15} style={{ color: 'var(--color-text-muted)', flexShrink: 0 }} />
            <input
              type="text"
              placeholder="Buscar"
              data-testid="sidebar-search-input"
              onFocus={() => setSearchFocused(true)}
              onBlur={() => setSearchFocused(false)}
              onKeyDown={handleSearchKeyDown}
              onClick={e => e.stopPropagation()}
              readOnly
              style={{
                background: 'none',
                border: 'none',
                outline: 'none',
                flex: 1,
                fontSize: '13px',
                color: 'var(--color-text-primary)',
                padding: '0 10px',
                minWidth: 0,
                cursor: 'pointer',
              }}
              className="sidebar-search-input"
            />
            <kbd
              style={{
                fontSize: '10px',
                color: 'var(--color-text-muted)',
                background: 'var(--color-surface)',
                padding: '2px 6px',
                borderRadius: '6px',
                flexShrink: 0,
                fontFamily: 'inherit',
                border: '1px solid var(--color-border)',
              }}
            >
              {navigator.platform.includes('Mac') ? '⌘ K' : 'Ctrl K'}
            </kbd>
          </div>
        </div>
      )}

      {/* Navigation */}
      <nav
        style={{
          flex: 1,
          padding: collapsed ? 'var(--space-3)' : '0 var(--space-3) var(--space-3)',
          overflowY: 'auto',
          display: 'flex',
          flexDirection: 'column',
          gap: collapsed ? 'var(--space-2)' : 'var(--space-6)',
        }}
        className="sidebar-scroll"
      >
        {/* Journals */}
        <section>
          <GroupHeader label="Diarios" collapsed={collapsed} />
          <div style={{ display: 'flex', flexDirection: 'column', gap: collapsed ? 'var(--space-1)' : '2px' }}>
            <SidebarItem
              icon={<Calendar size={18} />}
              label="Diarios"
              href={`/journal/${today.url}`}
              active={location.pathname.startsWith('/journal/')}
              collapsed={collapsed}
              dataTestId="nav-journal"
            />
          </div>
        </section>

        {/* All Pages */}
        <section>
          <GroupHeader label="Browse" collapsed={collapsed} />
          <div style={{ display: 'flex', flexDirection: 'column', gap: collapsed ? 'var(--space-1)' : '2px' }}>
            <SidebarItem
              icon={<LayoutList size={18} />}
              label="Lista de páginas"
              href="/pages"
              active={location.pathname === '/pages'}
              collapsed={collapsed}
              dataTestId="nav-pages"
            />
            <SidebarItem
              icon={<Network size={18} />}
              label="Vista de Grafo"
              href="/graph"
              active={location.pathname === '/graph'}
              collapsed={collapsed}
              dataTestId="nav-graph"
            />
          </div>
        </section>

        {/* Favorites — DESIGN.md §4.1, F2 of quilt-fase2-ux-empty-states */}
        {!collapsed && (
          <section>
            <GroupHeader label="Favoritos" />
            {favoritePages.length === 0 ? (
              // F2: previously the section was HIDDEN when empty,
              // leaving the user with no signal that favorites even
              // exist. Now we show a friendly message explaining how
              // to add one (star button on any page header).
              <p
                data-testid="favorites-empty"
                style={{
                  padding: '0 var(--space-2)',
                  fontSize: '12px',
                  color: 'var(--color-text-disabled)',
                  fontStyle: 'italic',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-2)',
                }}
              >
                <Star size={14} style={{ flexShrink: 0 }} aria-hidden="true" />
                <span>Click the star on any page to favorite it</span>
              </p>
            ) : (
              <ul
                style={{
                  listStyle: 'none',
                  margin: 0,
                  padding: 0,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '2px',
                }}
              >
                {favoritePages.map((page) => (
                  <li key={page.id} style={{ position: 'relative' }}>
                    <SidebarItem
                      icon={<Star size={18} style={{ color: 'var(--color-warning)' }} fill="currentColor" />}
                      label={page.title || page.name}
                      href={`/page/${encodeURIComponent(page.name)}`}
                      collapsed={collapsed}
                    />
                    <button
                      onClick={() => toggleFavorite(page.name)}
                      aria-label={`Remove ${page.name} from favorites`}
                      title="Remove from favorites"
                      style={{
                        position: 'absolute',
                        right: 'var(--space-2)',
                        top: '50%',
                        transform: 'translateY(-50%)',
                        background: 'transparent',
                        border: 'none',
                        cursor: 'pointer',
                        color: 'var(--color-text-disabled)',
                        padding: '2px 4px',
                        borderRadius: 'var(--radius-sm)',
                        fontSize: '11px',
                        opacity: 0,
                        transition: 'opacity var(--motion-fast) var(--ease-standard)',
                      }}
                      className="favorite-remove-btn"
                      onMouseEnter={(e) => { e.currentTarget.style.opacity = '1' }}
                      onMouseLeave={(e) => { e.currentTarget.style.opacity = '0' }}
                    >
                      <X size={12} />
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
        )}

        {/* Pages — F1 of quilt-fase2-ux-empty-states */}
        <section>
          <GroupHeader label="Páginas" collapsed={collapsed} />

          {loading ? (
            <SidebarSkeleton />
          ) : regularPages.length === 0 ? (
            !collapsed && (
              <p
                data-testid="pages-empty"
                style={{
                  padding: '0 var(--space-2)',
                  fontSize: '12px',
                  color: 'var(--color-text-disabled)',
                  fontStyle: 'italic',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 'var(--space-1)',
                }}
              >
                <span>No hay páginas todavía</span>
                <Link
                  to="/pages"
                  data-testid="pages-empty-see-all"
                  style={{
                    fontSize: '12px',
                    color: 'var(--color-primary)',
                    textDecoration: 'none',
                  }}
                  className="sidebar-item"
                >
                  Ver todas
                </Link>
              </p>
            )
          ) : (
            <>
              <ul
                style={{
                  listStyle: 'none',
                  margin: 0,
                  padding: 0,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: collapsed ? 'var(--space-1)' : '2px',
                }}
              >
                {/* F1: when there are more than 5 pages, only show
                    the 5 most recent ones and add a "Ver todas"
                    link at the bottom. Recent = newest by
                    `createdAt` desc. Below the threshold we just
                    show them all — the link would be noise. */}
                {regularPages
                  .slice()
                  .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
                  .slice(0, regularPages.length > 5 ? 5 : regularPages.length)
                  .map((page) => (
                    <li key={page.id}>
                      <SidebarItem
                        icon={<FileText size={18} />}
                        label={page.title || page.name}
                        href={`/page/${encodeURIComponent(page.name)}`}
                        collapsed={collapsed}
                      />
                    </li>
                  ))}
              </ul>
              {regularPages.length > 5 && !collapsed && (
                <Link
                  to="/pages"
                  data-testid="pages-see-all"
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 'var(--space-1)',
                    marginTop: 'var(--space-1)',
                    padding: '0 var(--space-2)',
                    fontSize: '12px',
                    fontWeight: 500,
                    color: 'var(--color-primary)',
                    textDecoration: 'none',
                  }}
                  className="sidebar-item"
                >
                  Ver todas ({regularPages.length})
                </Link>
              )}
            </>
          )}
        </section>

        {/* Templates — sidebar-template-ux (PR 3) */}
        <TemplateSection collapsed={collapsed} />

        {/* Recents — sidebar-recents (PR 2) */}
        <RecentsSection collapsed={collapsed} />

        {/* ADR-0003 — Agent Activity (cognitive feature, opt-in view) */}
        {!collapsed && showAgentActivity && (
          <section
            style={{
              borderTop: '1px solid var(--color-border)',
              paddingTop: 'var(--space-2)',
            }}
          >
            <AgentActivityPanel maxItems={15} />
          </section>
        )}
      </nav>

      {/* New page button */}
      <div
        style={{
          padding: collapsed ? 'var(--space-2)' : 'var(--space-3)',
          borderTop: '1px solid var(--color-border)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-1)',
        }}
      >
        <button
          onClick={handleNewPage}
          disabled={creating}
          aria-label="New page"
          title={collapsed ? 'New page' : undefined}
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: collapsed ? 'center' : 'center',
            gap: 'var(--space-2)',
            width: '100%',
            padding: collapsed ? '10px 0' : '12px var(--space-3)',
            borderRadius: 'var(--radius-md)',
            border: '1px solid var(--color-border)',
            background: 'var(--color-surface)',
            cursor: 'pointer',
            fontSize: '13px',
            fontWeight: 600,
            color: 'var(--color-primary)',
            boxShadow: 'var(--shadow-sm)',
            transition: 'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard), border-color var(--motion-fast) var(--ease-standard)',
          }}
          className="sidebar-item"
        >
          <Plus size={18} style={{ flexShrink: 0 }} />
          {!collapsed && (creating ? 'Creando…' : 'Nueva página')}
        </button>

        {/* ADR-0003 — Agent Activity toggle */}
        {/* F5 of quilt-fase2-ux-dead-buttons: relabeled the toggle
            from "Show / Hide agent activity" to a stable
            "Agent activity" with a tooltip that explains what it
            does. The previous labels changed on click, which was
            disorienting — the button moved around the layout when
            the text length changed. */}
        {!collapsed && (
          <button
            onClick={() => setShowAgentActivity(!showAgentActivity)}
            data-testid="agent-activity-toggle"
            aria-pressed={showAgentActivity}
            title="Show recent agent-authored blocks in the sidebar"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'flex-start',
              gap: 'var(--space-2)',
              width: '100%',
              padding: '6px var(--space-2)',
              borderRadius: 'var(--radius-md)',
              border: 'none',
              background: showAgentActivity
                ? 'var(--color-accent-subtle, rgba(99, 102, 241, 0.10))'
                : 'none',
              cursor: 'pointer',
              fontSize: '12px',
              fontWeight: 500,
              color: showAgentActivity
                ? 'var(--color-accent)'
                : 'var(--color-text-muted)',
              transition: 'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
            }}
            className="sidebar-item"
          >
            <Bot size={14} style={{ flexShrink: 0 }} />
            Agent activity
          </button>
        )}
      </div>
    </div>
  )
}
