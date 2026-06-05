// ─── RecentsSection ─────────────────────────────────────────────
//
// Tracks the last 5 visited pages via `useLocation()` and persists
// them to `localStorage` under `STORAGE_KEYS.RECENTS`. Spec:
// `openspec/specs/sidebar-recents/spec.md` (PR 2 of
// `quilt-fase1-sidebar-mcp-templates`).
//
// Behaviour (summary):
//   - On mount, reads the persisted list (with schema validation
//     and graceful fall-back for malformed JSON).
//   - On every route change, prepends an entry for `/page/:name`
//     routes. Non-page routes are ignored.
//   - Dedups by `name` case-insensitively, preserving the original
//     casing of the first occurrence.
//   - Caps the list at 5 — oldest is evicted on overflow.
//   - Clicking an item calls `api.getPage(name)` for self-heal;
//     404 removes the dead entry, shows a toast, and skips
//     navigation.

import { useEffect, useState } from 'react'
import { useLocation, useNavigate } from '@tanstack/react-router'
import { Clock, FileText } from 'lucide-react'
import toast from 'react-hot-toast'
import { api, QuiltApiError } from '@core/api-client'
import type { RecentPage } from '@shared/types/api'
import { STORAGE_KEYS } from '../storage-keys'
import { GroupHeader } from './GroupHeader'

const MAX_RECENTS = 5
const PAGE_PATH_REGEX = /^\/page\/(.+)$/

/** Type guard for a single recent-page entry shape. */
function isValidRecent(entry: unknown): entry is RecentPage {
  if (typeof entry !== 'object' || entry === null) return false
  const r = entry as Record<string, unknown>
  return (
    typeof r.name === 'string' &&
    typeof r.url === 'string' &&
    typeof r.visitedAt === 'number'
  )
}

/** Read the persisted list, with shape validation and a graceful fall-back.
 *  The result is always sorted by `visitedAt` descending so the render is
 *  stable regardless of the on-disk ordering. */
function readRecents(): RecentPage[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.RECENTS)
    if (!raw) return []
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter(isValidRecent)
      .sort((a, b) => b.visitedAt - a.visitedAt)
  } catch {
    // Malformed JSON or `localStorage` access denied — start empty.
    return []
  }
}

function writeRecents(recents: RecentPage[]) {
  // Persist in canonical order (newest first by `visitedAt`) so the
  // on-disk shape matches what the UI shows. This way, the list is
  // always sorted correctly even after a self-heal removal or a
  // cap-evicting prepend.
  const sorted = [...recents].sort((a, b) => b.visitedAt - a.visitedAt)
  localStorage.setItem(STORAGE_KEYS.RECENTS, JSON.stringify(sorted))
}

/**
 * Prepend `entry` to the list, removing any existing entry with
 * the same `name` (case-insensitive). If a match is found, the
 * existing entry's `name` and `url` are preserved — only the
 * `visitedAt` is refreshed (spec: "Re-visiting an entry SHALL move
 * it to the top with a fresh `visitedAt`"). Caps the result at
 * `MAX_RECENTS` entries.
 */
function prependRecent(recents: RecentPage[], entry: RecentPage): RecentPage[] {
  const idx = recents.findIndex(
    (r) => r.name.toLowerCase() === entry.name.toLowerCase(),
  )
  if (idx >= 0) {
    // Update visitedAt only; preserve original casing and URL.
    const updated: RecentPage = {
      ...recents[idx],
      visitedAt: entry.visitedAt,
    }
    const rest = [...recents.slice(0, idx), ...recents.slice(idx + 1)]
    return [updated, ...rest].slice(0, MAX_RECENTS)
  }
  return [entry, ...recents].slice(0, MAX_RECENTS)
}

interface RecentsSectionProps {
  collapsed: boolean
}

export function RecentsSection({ collapsed }: RecentsSectionProps) {
  const [recents, setRecents] = useState<RecentPage[]>(() => readRecents())
  const location = useLocation()
  const navigate = useNavigate()

  // Track route changes for `/page/:name` paths.
  useEffect(() => {
    const match = location.pathname.match(PAGE_PATH_REGEX)
    if (!match) return
    const name = decodeURIComponent(match[1])
    const entry: RecentPage = {
      name,
      url: location.pathname,
      visitedAt: Date.now(),
    }
    setRecents((prev) => {
      const next = prependRecent(prev, entry)
      writeRecents(next)
      return next
    })
  }, [location.pathname])

  async function handleClick(item: RecentPage) {
    try {
      await api.getPage(item.name)
    } catch (err) {
      // 404 → self-heal: drop the dead entry, toast, no navigation.
      const status =
        err instanceof QuiltApiError
          ? err.status
          : (err as { status?: number })?.status
      if (status === 404) {
        setRecents((prev) => {
          const next = prev.filter((r) => r.url !== item.url)
          writeRecents(next)
          return next
        })
        toast.error('Page not found')
        return
      }
      // Any other error: re-thrown would crash the click handler. Show
      // a generic message and skip navigation so the user stays put.
      toast.error(
        err instanceof Error ? err.message : 'Failed to open recent page',
      )
      return
    }
    navigate({ to: item.url as any })
  }

  if (collapsed) return null

  return (
    <section>
      <GroupHeader label="Recientes" />
      {recents.length === 0 ? (
        <div
          data-testid="recents-empty"
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
          <Clock size={14} />
          <span>Las páginas recientes aparecerán aquí</span>
        </div>
      ) : (
        <ul
          data-testid="recents-list"
          style={{
            listStyle: 'none',
            margin: 0,
            padding: 0,
            display: 'flex',
            flexDirection: 'column',
            gap: '2px',
          }}
        >
          {recents.map((item) => (
            <li key={item.url}>
              <button
                type="button"
                onClick={() => handleClick(item)}
                data-testid={`recent-${item.name}`}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-2)',
                  width: '100%',
                  padding: '10px var(--space-3)',
                  paddingLeft: 'calc(var(--space-2) + 3px)',
                  border: 'none',
                  borderRadius: '12px',
                  background: 'transparent',
                  cursor: 'pointer',
                  fontSize: '13px',
                  fontWeight: 400,
                  color: 'var(--color-text-secondary)',
                  textAlign: 'left',
                  minHeight: '40px',
                  transition:
                    'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
                }}
                className="sidebar-item"
              >
                <span
                  style={{
                    flexShrink: 0,
                    display: 'flex',
                    alignItems: 'center',
                  }}
                >
                  <FileText size={18} />
                </span>
                <span
                  style={{
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {item.name}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
