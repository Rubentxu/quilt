// ─── TemplateSection — sidebar templates UX (PR 3) ───────────────
//
// The sidebar's "Plantillas" section. Fetches `api.listTemplates()`
// once on mount (D6), renders one item per template, and on click
// creates a new page via `api.createPageFromTemplate({templateName,
// pageName})` and navigates to it.
//
// Template management (quilt-template-management-ui):
//   - Header has a "+" button (always visible) that opens the
//     CreateTemplateModal so users can author templates from the UI
//     instead of the terminal.
//   - The empty state has a more prominent "Create template" button
//     for the first-run experience.
//   - After a successful create, the section refetches the list so
//     the new template appears without a page reload.
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
//       sidebar-template-ux/spec.md (template list + create-from-template)
//      openspec/changes/quilt-template-management-ui/specs/
//       sidebar-template-create/spec.md (create-template flow)

import { useEffect, useState, useRef, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import toast from 'react-hot-toast'
import { FileText, Plus } from 'lucide-react'
import { api } from '@core/api-client'
import type { TemplateSummary } from '@shared/types/api'
import { GroupHeader } from './GroupHeader'
import { SidebarSkeleton } from './SidebarSkeleton'
import { CreateTemplateModal } from './CreateTemplateModal'

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
  // `quilt-template-management-ui` — controls the create-template modal.
  const [createOpen, setCreateOpen] = useState(false)
  const navigate = useNavigate()
  // `mountedRef` guarantees we never call setState after unmount even
  // if the `cancelled` closure variable is shadowed by re-renders.
  const mountedRef = useRef(true)

  // ── Fetch (D6 — single fetch on mount + refetch on demand) ───────
  //
  // `fetchTemplates` is reused by the create-template flow to refresh
  // the list after a new template is created (quilt-template-management-ui).
  // The mount effect calls it once; the unmount cleanup flags the
  // in-flight promise as stale so we never call setState after unmount.
  const fetchTemplates = useCallback(() => {
    let cancelled = false
    api
      .listTemplates()
      .then((data) => {
        if (cancelled || !mountedRef.current) return
        setTemplates(data)
        setLoading(false)
        // Clear any prior load error — the new fetch succeeded.
        setError(null)
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
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    const cleanup = fetchTemplates()
    return () => {
      mountedRef.current = false
      cleanup?.()
    }
  }, [fetchTemplates])

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
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingRight: 'var(--space-2)',
        }}
      >
        <GroupHeader label="Plantillas" />
        <button
          type="button"
          onClick={() => setCreateOpen(true)}
          aria-label="New template"
          title="New template"
          data-testid="template-create-header"
          style={{
            background: 'transparent',
            border: 'none',
            padding: '4px',
            marginBottom: 'var(--space-2)',
            borderRadius: 'var(--radius-sm)',
            cursor: 'pointer',
            color: 'var(--color-text-muted)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition:
              'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
          }}
          onMouseEnter={(e) => {
            ;(e.currentTarget as HTMLButtonElement).style.background =
              'var(--color-surface-subtle)'
            ;(e.currentTarget as HTMLButtonElement).style.color =
              'var(--color-text-primary)'
          }}
          onMouseLeave={(e) => {
            ;(e.currentTarget as HTMLButtonElement).style.background =
              'transparent'
            ;(e.currentTarget as HTMLButtonElement).style.color =
              'var(--color-text-muted)'
          }}
        >
          <Plus size={14} />
        </button>
      </div>

      {loading ? (
        <SidebarSkeleton />
      ) : templates.length === 0 ? (
        <div
          style={{
            padding: '0 var(--space-3)',
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-2)',
          }}
        >
          <p
            style={{
              margin: 0,
              fontSize: '12px',
              color: 'var(--color-text-disabled)',
              fontStyle: 'italic',
            }}
          >
            No templates available{error ? ' — could not load' : ''}
          </p>
          <button
            type="button"
            onClick={() => setCreateOpen(true)}
            data-testid="template-create-empty"
            aria-label="Create template"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '6px',
              padding: '8px var(--space-3)',
              border: '1px dashed var(--color-border)',
              borderRadius: 'var(--radius-md)',
              background: 'transparent',
              color: 'var(--color-text-secondary)',
              fontSize: '12px',
              fontWeight: 500,
              cursor: 'pointer',
              transition:
                'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
            }}
          >
            <Plus size={14} />
            Create template
          </button>
        </div>
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

      <CreateTemplateModal
        isOpen={createOpen}
        onClose={() => setCreateOpen(false)}
        onCreated={() => {
          // The modal already showed the success toast; we just need
          // to refetch the list so the new template is visible. The
          // modal's own onClose (called after onCreated internally)
          // dismisses the dialog — the order in the modal is:
          //   onCreated?.(fullName); onClose();
          // so this fires BEFORE the close, which is what we want.
          fetchTemplates()
        }}
      />
    </section>
  )
}
