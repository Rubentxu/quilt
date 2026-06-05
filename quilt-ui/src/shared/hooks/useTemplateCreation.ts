/**
 * useTemplateCreation — "New from Template" wizard hook (F15, ADR-0003).
 *
 * Extracted from `BlockRow.handleInsertTemplate` (architecture review
 * candidate #5). Owns the multi-step flow that used to live inside a
 * block renderer:
 *
 *   1. Fetch `api.listTemplates()` on mount.
 *   2. `createFromTemplate(pageName, originalContent)` —
 *      a. If pageName is empty → cancel + restore.
 *      b. If no templates → error toast + restore.
 *      c. If one template → use it.
 *      d. If multiple → prompt user to pick one.
 *      e. Call `api.createPageFromTemplate(...)` → success toast +
 *         navigate to the new page.
 *      f. On any cancel/error → call `onRestore(originalContent)`.
 *
 * Returns a discriminated `Result` instead of throwing so callers can
 * branch on `reason` without `try/catch`.
 */

import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import toast from 'react-hot-toast'
import { api } from '@core/api-client'
import type { Page, TemplateSummary } from '@shared/types/api'

/** Discriminated union returned from `createFromTemplate`. */
export type CreateFromTemplateResult =
  | { ok: true; page: Page; blocksCreated: number }
  | {
      ok: false
      reason: 'cancelled' | 'no_templates' | 'template_not_found' | 'api_error'
      error: string
    }

export interface UseTemplateCreationOptions {
  /**
   * Called on every cancel / error path with the `originalContent`
   * the caller passed to `createFromTemplate`. Lets the caller
   * restore the block's text on its own DOM (e.g. by writing back to
   * the contenteditable ref). The hook itself stays DOM-agnostic.
   */
  onRestore?: (originalContent: string) => void
}

export interface UseTemplateCreationResult {
  /**
   * Run the wizard. `pageName` is the new page's name (already
   * trimmed by the caller). `originalContent` is forwarded to
   * `onRestore` if the user bails before the page is created — this
   * preserves the leading "/template" text the user typed.
   */
  createFromTemplate: (pageName: string, originalContent: string) => Promise<CreateFromTemplateResult>
  /** All templates fetched on mount, in server order. */
  templates: TemplateSummary[]
  /** `true` while the initial `listTemplates` call is in flight. */
  loading: boolean
  /** Last error from the initial fetch (not from createFromTemplate). */
  error: string | null
}

export function useTemplateCreation(
  options: UseTemplateCreationOptions = {},
): UseTemplateCreationResult {
  const { onRestore } = options
  const navigate = useNavigate()
  const [templates, setTemplates] = useState<TemplateSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Fetch templates once on mount. We don't expose a refresh API yet
  // — the slash command's "New from Template" is rare enough that a
  // stale list is acceptable, and the user can re-open the slash
  // menu to pick a freshly-created template.
  useEffect(() => {
    let cancelled = false
    setLoading(true)
    api
      .listTemplates()
      .then(list => {
        if (cancelled) return
        setTemplates(list)
        setError(null)
      })
      .catch(err => {
        if (cancelled) return
        const message = err instanceof Error ? err.message : String(err)
        setError(message)
        setTemplates([])
      })
      .finally(() => {
        if (cancelled) return
        setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const createFromTemplate = useCallback(
    async (
      pageName: string,
      originalContent: string,
    ): Promise<CreateFromTemplateResult> => {
      const restore = () => onRestore?.(originalContent)

      // 1. Empty page name → cancel, restore original content.
      if (!pageName || !pageName.trim()) {
        const message = 'Page name is required'
        restore()
        return { ok: false, reason: 'cancelled', error: message }
      }

      const trimmed = pageName.trim()

      try {
        // 2. Empty template list → user error, restore.
        if (templates.length === 0) {
          const message = 'No templates found. Create one in the Plantillas section first.'
          toast.error(message)
          restore()
          return { ok: false, reason: 'no_templates', error: message }
        }

        // 3. Pick the template — auto-pick if only one.
        let template = templates[0]
        if (templates.length > 1) {
          const labels = templates.map(t => t.name).join(', ')
          const choice = window.prompt(
            `Choose template (${labels}):`,
            templates[0].name,
          )
          if (!choice || !choice.trim()) {
            restore()
            return {
              ok: false,
              reason: 'cancelled',
              error: 'Template selection cancelled',
            }
          }
          const picked = templates.find(t => t.name === choice.trim())
          if (!picked) {
            const message = `Template not found: ${choice}`
            toast.error(message)
            restore()
            return { ok: false, reason: 'template_not_found', error: message }
          }
          template = picked
        }

        // 4. Call the server endpoint. Use `full_name` (with the
        // `template/` prefix) — the server requires it.
        const result = await api.createPageFromTemplate({
          templateName: template.full_name,
          pageName: trimmed,
          title: trimmed,
        })

        toast.success(
          `Created from template "${template.name}" (${result.blocksCreated} blocks)`,
        )

        // 5. Navigate — block content becomes irrelevant.
        navigate({ to: '/page/$name', params: { name: result.page.name } })

        return { ok: true, page: result.page, blocksCreated: result.blocksCreated }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        const full = `Failed to create from template: ${message}`
        toast.error(full)
        restore()
        return { ok: false, reason: 'api_error', error: full }
      }
    },
    [templates, navigate, onRestore],
  )

  return { createFromTemplate, templates, loading, error }
}
