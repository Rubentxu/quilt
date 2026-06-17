/**
 * GraphSelectorPage — `/select-graph` route (ADR-0030, Slice D).
 *
 * Provides three ways to open or create a graph:
 * 1. Open recent: click a recent graph from the list
 * 2. Open by path: enter or pick a directory path
 * 3. Create new: same as #2 (server is idempotent)
 *
 * Per ADR-0030 §8, this is NOT the home — the home always lands
 * on today's journal when a valid last_opened_graph exists.
 */

import { useEffect, useRef, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { api, QuiltApiError } from '@core/api-client'

interface GlobalState {
  lastOpenedGraph: string | null
  recentGraphs: string[]
  rightSidebarVisible: boolean | null
}

interface ValidationError {
  code: string
  validationError: string
  path: string
}

type ErrorDisplay =
  | { type: 'none' }
  | { type: 'network'; message: string }
  | { type: 'validation'; error: ValidationError }

/** A recent graph entry with cached display name from graph_space metadata */
interface RecentGraphEntry {
  path: string
  name: string | null
}

/**
 * Derive a display name from a graph path (fallback when metadata unavailable).
 * Uses the last path segment as the display name.
 */
function deriveNameFromPath(path: string): string {
  const parts = path.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || path
}

function formatTodayDate(): string {
  const now = new Date()
  const y = now.getFullYear()
  const m = String(now.getMonth() + 1).padStart(2, '0')
  const d = String(now.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

export function GraphSelectorPage() {
  const navigate = useNavigate()
  const [recentGraphs, setRecentGraphs] = useState<RecentGraphEntry[]>([])
  const [pathInput, setPathInput] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<ErrorDisplay>({ type: 'none' })
  const [activeAction, setActiveAction] = useState<'recent' | 'open' | 'create'>('recent')

  // Focus management: focus the path input when the "open" or "create" tab activates
  const pathInputRef = useRef<HTMLInputElement>(null)
  useEffect(() => {
    if (activeAction === 'open' || activeAction === 'create') {
      pathInputRef.current?.focus()
    }
  }, [activeAction])

  // Load recent graphs on mount (GS-7: fetch graph_space metadata.name for display)
  useEffect(() => {
    api.getGlobalState().then(async (state: GlobalState) => {
      const paths = state.recentGraphs ?? []
      // For each recent graph, try to fetch its metadata.name
      // We initialize with path-derived names and update if metadata is available
      const entries: RecentGraphEntry[] = paths.map((path) => ({
        path,
        name: deriveNameFromPath(path),
      }))
      setRecentGraphs(entries)

      // Try to fetch graph_space metadata for each path to get the proper name
      // This is best-effort - if it fails, we use the path-derived name
      await Promise.allSettled(
        paths.map(async (graphPath, index) => {
          try {
            // We need to open the graph first to have a valid DB connection
            // For recently opened graphs, we assume the path is still valid
            const gs = await api.getGraphSpace()
            // Update the name if we got a valid response
            setRecentGraphs((prev) =>
              prev.map((entry, i) =>
                i === index ? { ...entry, name: gs.name } : entry
              )
            )
          } catch {
            // Non-fatal: keep using the path-derived name
          }
        })
      )
    }).catch(() => {
      // Non-fatal: recent list just stays empty
    })
  }, [])

  async function openGraph(graphPath: string) {
    setLoading(true)
    setError({ type: 'none' })
    try {
      // Validate (idempotent) and open via create endpoint
      const result = await api.createGraph(graphPath)
      if (result.created) {
        // New graph created — navigate to today's journal
        navigate({ to: '/journal/$date', params: { date: formatTodayDate() } })
      } else {
        // Existing graph opened — navigate to today's journal
        navigate({ to: '/journal/$date', params: { date: formatTodayDate() } })
      }
    } catch (err) {
      if (err instanceof QuiltApiError && err.status === 422) {
        setError({
          type: 'validation',
          error: {
            code: err.code,
            validationError: err.detail,
            path: graphPath,
          },
        })
      } else {
        setError({ type: 'network', message: String(err) })
      }
    } finally {
      setLoading(false)
    }
  }

  async function handleRecentClick(graphPath: string) {
    await openGraph(graphPath)
  }

  async function handleOpenByPath(e: React.FormEvent) {
    e.preventDefault()
    const path = pathInput.trim()
    if (!path) return
    await openGraph(path)
  }

  async function handleCreateNew(e: React.FormEvent) {
    e.preventDefault()
    const path = pathInput.trim()
    if (!path) return
    await openGraph(path)
  }

  function handlePickDirectory() {
    // Use the File System Access API if available
    if ('showDirectoryPicker' in window) {
      ;(window as any)
        .showDirectoryPicker()
        .then((handle: any) => {
          // The result is a FileSystemDirectoryHandle; we need the path
          // On Chrome/Edge with origin-private file system access,
          // we can get the path via `handle.getAsFileSystemRoot()`
          // but on Firefox/Safari it's not available.
          // Fall back: use the text input for now; in future we could
          // persist the handle to IndexedDB.
          if (handle.path) {
            setPathInput(handle.path)
          } else {
            // Can't derive a path from the handle on all platforms;
            // surface a hint to the user.
            setPathInput(
              'Directory picker available — enter the path manually or copy from the picker dialog'
            )
          }
        })
        .catch(() => {
          // User cancelled or picker unavailable — no-op
        })
    }
  }

  return (
    <div
      style={{
        maxWidth: '560px',
        margin: '0 auto',
        padding: 'var(--space-8) var(--space-4)',
      }}
    >
      <h1
        style={{
          fontSize: '1.5rem',
          fontWeight: 600,
          marginBottom: 'var(--space-6)',
          color: 'var(--color-text-primary)',
        }}
      >
        Open or Create a Graph
      </h1>

      {/* Error region — a11y */}
      <div
        aria-live="polite"
        aria-atomic="true"
        role="alert"
        style={{ marginBottom: 'var(--space-4)' }}
      >
        {error.type === 'validation' && (
          <div
            style={{
              padding: 'var(--space-3) var(--space-4)',
              background: 'color-mix(in srgb, var(--color-error) 10%, transparent)',
              border: '1px solid var(--color-error)',
              borderRadius: 'var(--radius-md)',
              color: 'var(--color-error)',
              fontSize: '0.875rem',
            }}
          >
            <strong>Cannot open:</strong> {error.error.validationError}
            {error.error.path && (
              <span
                style={{
                  display: 'block',
                  fontFamily: 'monospace',
                  fontSize: '0.75rem',
                  marginTop: 'var(--space-1)',
                  opacity: 0.8,
                }}
              >
                {error.error.path}
              </span>
            )}
          </div>
        )}
        {error.type === 'network' && (
          <div
            style={{
              padding: 'var(--space-3) var(--space-4)',
              background: 'color-mix(in srgb, var(--color-warning) 10%, transparent)',
              border: '1px solid var(--color-warning)',
              borderRadius: 'var(--radius-md)',
              color: 'var(--color-warning)',
              fontSize: '0.875rem',
            }}
          >
            <strong>Network error:</strong> {error.message}
          </div>
        )}
      </div>

      {/* Action tabs */}
      <div
        role="tablist"
        style={{
          display: 'flex',
          gap: 'var(--space-1)',
          marginBottom: 'var(--space-6)',
          borderBottom: '1px solid var(--color-border)',
          paddingBottom: 'var(--space-1)',
        }}
      >
        {(
          [
            { key: 'recent', label: 'Recent' },
            { key: 'open', label: 'Open by Path' },
            { key: 'create', label: 'Create New' },
          ] as const
        ).map(({ key, label }) => (
          <button
            key={key}
            role="tab"
            aria-selected={activeAction === key}
            onClick={() => setActiveAction(key)}
            style={{
              padding: 'var(--space-2) var(--space-4)',
              border: 'none',
              borderBottom:
                activeAction === key
                  ? '2px solid var(--color-link)'
                  : '2px solid transparent',
              background: 'transparent',
              color:
                activeAction === key
                  ? 'var(--color-link)'
                  : 'var(--color-text-secondary)',
              cursor: 'pointer',
              fontSize: '0.875rem',
              fontWeight: 500,
              transition: 'color 0.15s, border-color 0.15s',
            }}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Recent graphs */}
      {activeAction === 'recent' && (
        <div role="tabpanel" aria-label="Recent graphs">
          {recentGraphs.length === 0 ? (
            <div
              style={{
                textAlign: 'center',
                padding: 'var(--space-8) var(--space-4)',
                color: 'var(--color-text-muted)',
                fontSize: '0.875rem',
              }}
            >
              No recent graphs. Use "Open by Path" or "Create New" to get started.
            </div>
          ) : (
            <ul
              style={{
                listStyle: 'none',
                padding: 0,
                margin: 0,
                display: 'flex',
                flexDirection: 'column',
                gap: 'var(--space-2)',
              }}
            >
              {recentGraphs.map((entry) => (
                <li key={entry.path}>
                  <button
                    type="button"
                    onClick={() => handleRecentClick(entry.path)}
                    disabled={loading}
                    style={{
                      width: '100%',
                      textAlign: 'left',
                      padding: 'var(--space-3) var(--space-4)',
                      background: 'var(--color-surface-elevated)',
                      border: '1px solid var(--color-border)',
                      borderRadius: 'var(--radius-md)',
                      cursor: loading ? 'not-allowed' : 'pointer',
                      opacity: loading ? 0.6 : 1,
                      transition: 'opacity 0.15s, border-color 0.15s, background 0.15s',
                    }}
                    onMouseEnter={(e) => {
                      if (!loading) {
                        e.currentTarget.style.borderColor = 'var(--color-link)'
                        e.currentTarget.style.background = 'var(--color-surface-subtle)'
                      }
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.borderColor = 'var(--color-border)'
                      e.currentTarget.style.background = 'var(--color-surface-elevated)'
                    }}
                  >
                    <span
                      style={{
                        display: 'block',
                        fontWeight: 500,
                        color: 'var(--color-text-primary)',
                        fontSize: '0.875rem',
                      }}
                    >
                      {entry.name}
                    </span>
                    <span
                      style={{
                        display: 'block',
                        fontSize: '0.75rem',
                        color: 'var(--color-text-muted)',
                        fontFamily: 'monospace',
                        marginTop: '2px',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {entry.path}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      {/* Open by path */}
      {activeAction === 'open' && (
        <div role="tabpanel" aria-label="Open by path">
          <form onSubmit={handleOpenByPath} style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
            <div>
              <label
                htmlFor="graph-path-open"
                style={{
                  display: 'block',
                  fontSize: '0.875rem',
                  fontWeight: 500,
                  marginBottom: 'var(--space-2)',
                  color: 'var(--color-text-primary)',
                }}
              >
                Graph directory path
              </label>
              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                <input
                  id="graph-path-open"
                  ref={pathInputRef}
                  type="text"
                  value={pathInput}
                  onChange={(e) => setPathInput(e.target.value)}
                  placeholder="/home/user/my-graph"
                  style={{
                    flex: 1,
                    padding: 'var(--space-2) var(--space-3)',
                    border: '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-md)',
                    fontSize: '0.875rem',
                    fontFamily: 'monospace',
                    background: 'var(--color-surface)',
                    color: 'var(--color-text-primary)',
                    outline: 'none',
                  }}
                />
                {'showDirectoryPicker' in window && (
                  <button
                    type="button"
                    onClick={handlePickDirectory}
                    style={{
                      padding: 'var(--space-2) var(--space-3)',
                      border: '1px solid var(--color-border)',
                      borderRadius: 'var(--radius-md)',
                      background: 'var(--color-surface-subtle)',
                      cursor: 'pointer',
                      fontSize: '0.875rem',
                      color: 'var(--color-text-secondary)',
                    }}
                  >
                    Pick…
                  </button>
                )}
              </div>
            </div>
            <button
              type="submit"
              disabled={loading || !pathInput.trim()}
              style={{
                padding: 'var(--space-2) var(--space-4)',
                background: loading ? 'var(--color-surface-subtle)' : 'var(--color-link)',
                color: '#fff',
                border: 'none',
                borderRadius: 'var(--radius-md)',
                cursor: loading || !pathInput.trim() ? 'not-allowed' : 'pointer',
                fontSize: '0.875rem',
                fontWeight: 500,
                opacity: loading || !pathInput.trim() ? 0.6 : 1,
                transition: 'opacity 0.15s',
                alignSelf: 'flex-start',
              }}
            >
              {loading ? 'Opening…' : 'Open Graph'}
            </button>
          </form>
        </div>
      )}

      {/* Create new */}
      {activeAction === 'create' && (
        <div role="tabpanel" aria-label="Create new graph">
          <form onSubmit={handleCreateNew} style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
            <p
              style={{
                fontSize: '0.875rem',
                color: 'var(--color-text-secondary)',
                marginBottom: 'var(--space-2)',
              }}
            >
              Enter or pick an empty directory. Quilt will create a{' '}
              <code style={{ fontSize: '0.75rem' }}>.quilt/quilt.db</code> inside it.
            </p>
            <div>
              <label
                htmlFor="graph-path-create"
                style={{
                  display: 'block',
                  fontSize: '0.875rem',
                  fontWeight: 500,
                  marginBottom: 'var(--space-2)',
                  color: 'var(--color-text-primary)',
                }}
              >
                New graph directory path
              </label>
              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                <input
                  id="graph-path-create"
                  ref={pathInputRef}
                  type="text"
                  value={pathInput}
                  onChange={(e) => setPathInput(e.target.value)}
                  placeholder="/home/user/new-graph"
                  style={{
                    flex: 1,
                    padding: 'var(--space-2) var(--space-3)',
                    border: '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-md)',
                    fontSize: '0.875rem',
                    fontFamily: 'monospace',
                    background: 'var(--color-surface)',
                    color: 'var(--color-text-primary)',
                    outline: 'none',
                  }}
                />
                {'showDirectoryPicker' in window && (
                  <button
                    type="button"
                    onClick={handlePickDirectory}
                    style={{
                      padding: 'var(--space-2) var(--space-3)',
                      border: '1px solid var(--color-border)',
                      borderRadius: 'var(--radius-md)',
                      background: 'var(--color-surface-subtle)',
                      cursor: 'pointer',
                      fontSize: '0.875rem',
                      color: 'var(--color-text-secondary)',
                    }}
                  >
                    Pick…
                  </button>
                )}
              </div>
            </div>
            <button
              type="submit"
              disabled={loading || !pathInput.trim()}
              style={{
                padding: 'var(--space-2) var(--space-4)',
                background: loading ? 'var(--color-surface-subtle)' : 'var(--color-success)',
                color: '#fff',
                border: 'none',
                borderRadius: 'var(--radius-md)',
                cursor: loading || !pathInput.trim() ? 'not-allowed' : 'pointer',
                fontSize: '0.875rem',
                fontWeight: 500,
                opacity: loading || !pathInput.trim() ? 0.6 : 1,
                transition: 'opacity 0.15s',
                alignSelf: 'flex-start',
              }}
            >
              {loading ? 'Creating…' : 'Create Graph'}
            </button>
          </form>
        </div>
      )}
    </div>
  )
}