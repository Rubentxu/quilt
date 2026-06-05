// ─── TemplateSection — sidebar templates UX (PR 3) ───────────────
//
// The sidebar's "Plantillas" section. Fetches `api.listTemplates()`
// once on mount (D6), renders one item per template, and on click
// creates a new page via `api.createPageFromTemplate({templateName,
// pageName})` and navigates to it.
//
// Design constraints:
//   - D5: section is positioned after Pages in the sidebar nav.
//   - D6: single fetch on mount; cancellation on unmount via the
//     `cancelled` flag pattern (the api-client itself doesn't yet
//     take an `AbortSignal` for listTemplates).
//   - A11y (DESIGN.md §9.1 + spec): keyboard activatable
//     (Enter/Space on a `<button>` is automatic), `aria-label`
//     "Create page from template: <name>".
//   - Loading: shares the same `SidebarSkeleton` used by the Pages
//     section for visual stability.
//   - Error: `toast.error(...)` and falls back to the empty state so
//     the section never blocks the rest of the sidebar.
//
// Spec: openspec/changes/quilt-fase1-sidebar-mcp-templates/specs/
//       sidebar-template-ux/spec.md

import { useEffect, useState, useRef, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import toast from 'react-hot-toast'
import { FileText } from 'lucide-react'
import { api } from '@core/api-client'
import type { TemplateSummary } from '@shared/types/api'
import { GroupHeader } from './GroupHeader'
import { SidebarSkeleton } from './SidebarSkeleton'

interface TemplateSectionProps {
  /** When true, the section is hidden (matches existing sidebar collapsed UX). */
  collapsed?: boolean
}

export function TemplateSection({ collapsed }: TemplateSectionProps) {
  const [templates, setTemplates] = useState<TemplateSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<Error | null>(null)
  // Track which template the user is currently creating from so the
  // matching item is disabled and ignores further clicks (spec:
  // "Click on busy item is ignored"). Using a Set keeps the type
  // honest when the API allows multiple in-flight creates.
  const [busyNames, setBusyNames] = useState<Set<string>>(new Set())
  const navigate = useNavigate()
  // `mountedRef` guarantees we never call setState after unmount even
  // if the `cancelled` closure variable is shadowed by re-renders.
  const mountedRef = useRef(true)

  // ── Fetch on mount (D6) ───────────────────────────────────────────
  useEffect(() => {
    mountedRef.current = true
    let cancelled = false
    api
      .listTemplates()
      .then((data) => {
        if (cancelled || !mountedRef.current) return
        setTemplates(data)
        setLoading(false)
      })
      .catch((err: unknown) => {
        if (cancelled || !mountedRef.current) return
        const message = err instanceof Error ? err.message : 'Unknown error'
        toast.error(`Failed to load templates: ${message}`)
        setError(err instanceof Error ? err : new Error(String(err)))
        setLoading(false)
      })
    return () => {
      cancelled = true
      mountedRef.current = false
    }
  }, [])

  // ── Click handler — create + navigate (spec) ──────────────────────
  const handleClick = useCallback(
    async (template: TemplateSummary) => {
      if (busyNames.has(template.name)) return
      setBusyNames((prev) => {
        const next = new Set(prev)
        next.add(template.name)
        return next
      })
      try {
        const pageName = `${template.name}-1`
        // Spec: CreatePageFromTemplateRequest.templateName must start with
        // `template/` — the server resolves templates by their full name
        // (`template/<short>`), not by the short name alone. Passing
        // `template.name` (e.g. "my-template") produces a 404 in production;
        // the unit suite did not catch this because the API is mocked.
        // Regression test: TemplateSection.test.tsx > 'passes template.full_name...'.
        const result = await api.createPageFromTemplate({
          templateName: template.full_name,
          pageName,
        })
        if (!mountedRef.current) return
        navigate({ to: `/page/${encodeURIComponent(result.page.name)}` })
      } catch (err) {
        if (!mountedRef.current) return
        const message = err instanceof Error ? err.message : 'Unknown error'
        toast.error(`Failed to create page from template: ${message}`)
      } finally {
        if (mountedRef.current) {
          setBusyNames((prev) => {
            const next = new Set(prev)
            next.delete(template.name)
            return next
          })
        }
      }
    },
    [busyNames, navigate],
  )

  // ── Hidden when sidebar is collapsed (spec) ───────────────────────
  if (collapsed) return null

  return (
    <section>
      <GroupHeader label="Plantillas" />

      {loading ? (
        <SidebarSkeleton />
      ) : templates.length === 0 ? (
        <p
          style={{
            padding: '0 var(--space-2)',
            fontSize: '12px',
            color: 'var(--color-text-disabled)',
            fontStyle: 'italic',
          }}
        >
          No templates available{error ? ' — could not load' : ''}
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
          {templates.map((tpl) => {
            const busy = busyNames.has(tpl.name)
            return (
              <li key={tpl.name}>
                <button
                  type="button"
                  data-testid={`template-item-${tpl.name}`}
                  onClick={() => handleClick(tpl)}
                  disabled={busy}
                  aria-busy={busy || undefined}
                  aria-label={`Create page from template: ${tpl.name}`}
                  className="sidebar-item"
                  style={{
                    position: 'relative',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 'var(--space-2)',
                    padding: '10px var(--space-3)',
                    paddingLeft: 'calc(var(--space-2) + 3px)',
                    borderRadius: '12px',
                    border: 'none',
                    background: 'transparent',
                    cursor: busy ? 'wait' : 'pointer',
                    textAlign: 'left',
                    width: '100%',
                    fontSize: '13px',
                    fontWeight: 400,
                    color: 'var(--color-text-secondary)',
                    minHeight: '40px',
                    transition:
                      'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
                    opacity: busy ? 0.6 : 1,
                  }}
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
                    {tpl.name}
                  </span>
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
