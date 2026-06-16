/**
 * QueryBuilder — F17 query builder container (CG-4 enhanced).
 *
 * Orchestrates QueryInput + QuerySnippets + QueryResults into a
 * cohesive query experience:
 *
 * - QueryInput: dual-mode (DSL text input + FilterChipGroup chips)
 * - QuerySnippets: predefined DSL query templates
 * - QueryResults: interactive results with keyboard navigation
 * - queryHistory: recent DSL queries in localStorage
 * - executeQuery via api.executeQuery
 * - AbortController cancels any in-flight request before starting a new one
 *
 * Layout:
 *   ┌─ QueryInput (DSL / chips toggle) ──────────────────────┐
 *   │  [(chips)] [textarea...                    ] [Run] [⌄]   │
 *   └──────────────────────────────────────────────────────────┘
 *   ┌─ QuerySnippets (collapsible) ───────┬─ QueryResults ──┐
 *   │  Journal                            │  ↑↓ navigate    │
 *   │    • Journal this week              │  Enter navigate │
 *   │    • Journal today                  │  Space expand   │
 *   │  Tasks                              │                 │
 *   │    • Tasks scheduled today          │  Results list   │
 *   │    • All TODO tasks                 │                 │
 *   └─────────────────────────────────────┴─────────────────┘
 */

import { useCallback, useRef, useState } from 'react'
import { QueryInput } from './QueryInput'
import { QuerySnippets } from './QuerySnippets'
import { QueryResults } from './QueryResults'
import { TableView } from '../table-view/TableView'
import type { ColumnDef } from '../table-view/ColumnDef'
import type { FilterChip } from '@shared/types/filterChip'
import type { QueryAst, QueryResult, SortDirection } from '@shared/types/queryAst'
import { buildQueryAst } from '@shared/utils/buildQueryAst'
import { api } from '@core/api-client'
import { validateChipList } from '@shared/types/filterChip'
import {
  loadQueryHistory,
  recordQueryExecution,
  type QueryHistoryEntry,
} from './queryHistory'

export interface QueryBuilderProps {
  /** Column definitions for the TableView (required for sort support). */
  columns: ColumnDef[]
  /** Available property keys for the Add Filter dropdown. */
  availableKeys?: string[]
  /** Default limit for query results. @default 100 */
  defaultLimit?: number
  /** Called when results are returned (includes elapsed_ms for perf display). */
  onResults?: (result: QueryResult) => void
  /** Called when an error occurs. */
  onError?: (error: Error) => void
}

// Default columns when none provided
const DEFAULT_COLUMNS: ColumnDef[] = [
  { key: 'name', header: 'Name', width: 200 },
  { key: 'status', header: 'Status', width: 120, sortable: true },
]

// Default limit
const DEFAULT_LIMIT = 100

// ─── New types for CG-4 ─────────────────────────────────────────

interface QueryBuilderState {
  dslText: string
  chips: FilterChip[]
  history: QueryHistoryEntry[]
  showSnippets: boolean
  viewMode: 'results' | 'table'
}

/**
 * QueryBuilder — orchestrates DSL input + snippets + query execution + results.
 */
export function QueryBuilder({
  columns: columnsProp,
  availableKeys,
  defaultLimit = DEFAULT_LIMIT,
  onResults,
  onError,
}: QueryBuilderProps) {
  const columns = columnsProp ?? DEFAULT_COLUMNS

  // ─── State ─────────────────────────────────────────────────────────────────

  const [state, setState] = useState<QueryBuilderState>({
    dslText: '',
    chips: [],
    history: loadQueryHistory(),
    showSnippets: true,
    viewMode: 'results',
  })

  const [result, setResult] = useState<QueryResult | null>(null)
  const [loading, setLoading] = useState(false)
  const [executeError, setExecuteError] = useState<Error | null>(null)

  // ─── Abort controller for request cancellation ─────────────────────────────

  const abortControllerRef = useRef<AbortController | null>(null)

  const cancelPrevious = useCallback(() => {
    abortControllerRef.current?.abort()
    abortControllerRef.current = new AbortController()
  }, [])

  // ─── Query execution ───────────────────────────────────────────────────────

  /**
   * Execute a query AST with an optional sort override.
   * Cancels any in-flight request before starting.
   */
  const execute = useCallback(
    async (
      dsl: string,
      ast: QueryAst | null,
      sortKey?: string,
      sortDir?: SortDirection,
    ) => {
      cancelPrevious()
      setLoading(true)
      setExecuteError(null)

      const controller = abortControllerRef.current!

      try {
        if (!ast) {
          setResult({ results: [], total: 0, elapsed_ms: 0 })
          return
        }

        const effectiveAst: QueryAst = sortKey
          ? { SortBy: { field: sortKey, direction: sortDir ?? 'Asc', inner: ast } }
          : ast

        const queryResult = await api.executeQuery(
          effectiveAst,
          defaultLimit,
          controller.signal,
        )

        if (!controller.signal.aborted) {
          setResult(queryResult)
          onResults?.(queryResult)

          // Record to query history
          if (dsl.trim()) {
            const updated = recordQueryExecution(
              { query: dsl, resultCount: queryResult.total },
              Date.now(),
            )
            setState(prev => ({ ...prev, history: updated }))
          }
        }
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
          return
        }
        const error = err instanceof Error ? err : new Error(String(err))
        setExecuteError(error)
        onError?.(error)
      } finally {
        if (!controller.signal.aborted) {
          setLoading(false)
        }
      }
    },
    [cancelPrevious, defaultLimit, onResults, onError],
  )

  // ─── DSL execute ───────────────────────────────────────────────────────────

  function handleDslExecute(dsl: string, ast: QueryAst | null) {
    execute(dsl, ast)
  }

  // ─── Filter apply (chips mode) ─────────────────────────────────────────────

  function handleChipsApply(appliedChips: FilterChip[]) {
    const errors = validateChipList(appliedChips)
    if (Object.keys(errors).length > 0) return

    const ast = buildQueryAst(appliedChips)
    // Build a display DSL from chips
    const dsl = chipsToDsl(appliedChips)
    execute(dsl, ast)
  }

  // ─── Snippet insert ────────────────────────────────────────────────────────

  function handleSnippetInsert(dsl: string) {
    setState(prev => ({
      ...prev,
      dslText: dsl,
      showSnippets: false,
    }))
  }

  // ─── History execute ─────────────────────────────────────────────────────────

  function handleHistoryClick(entry: QueryHistoryEntry) {
    setState(prev => ({ ...prev, dslText: entry.query }))
    // Validate and execute
    import('@shared/utils/validateQuery').then(({ validateQuery }) => {
      validateQuery(entry.query).then(result => {
        if (result.valid && result.ast) {
          execute(entry.query, result.ast)
        }
      })
    })
  }

  // ─── Sort ───────────────────────────────────────────────────────────────────

  function handleSort(sortKey: string, sortDir: SortDirection) {
    const chips = state.chips
    const ast = buildQueryAst(chips)
    const dsl = chipsToDsl(chips)
    execute(dsl, ast, sortKey, sortDir)
  }

  // ─── Render ───────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col gap-4" data-testid="query-builder">
      {/* ─── Query input (DSL + chips toggle) ─── */}
      <div data-testid="query-builder-input">
        <QueryInput
          value={state.dslText}
          onChange={text =>
            setState(prev => ({ ...prev, dslText: text }))
          }
          onExecute={handleDslExecute}
          availableKeys={availableKeys}
          disabled={loading}
          error={executeError ? { message: executeError.message } : null}
          chips={state.chips}
          onChipsChange={chips =>
            setState(prev => ({ ...prev, chips }))
          }
          onChipsApply={handleChipsApply}
          initialMode="dsl"
        />
      </div>

      {/* ─── Main content: snippets + results ─── */}
      <div
        style={{
          display: 'flex',
          gap: 'var(--space-4)',
          alignItems: 'flex-start',
        }}
      >
        {/* ─── Snippets sidebar ─── */}
        {state.showSnippets && (
          <div
            data-testid="query-builder-snippets"
            style={{
              width: '240px',
              flexShrink: 0,
              maxHeight: '450px',
              overflowY: 'auto',
              borderRight: '1px solid var(--color-border)',
              paddingRight: 'var(--space-3)',
            }}
          >
            {/* History section */}
            {state.history.length > 0 && (
              <div style={{ marginBottom: 'var(--space-3)' }}>
                <div
                  style={{
                    fontSize: 'var(--font-size-micro)',
                    fontWeight: 600,
                    textTransform: 'uppercase',
                    letterSpacing: 'var(--tracking-wider)',
                    color: 'var(--color-text-muted)',
                    marginBottom: 'var(--space-1)',
                  }}
                >
                  Recent ({state.history.length})
                </div>
                {state.history.slice(0, 5).map(entry => (
                  <button
                    key={`${entry.query}-${entry.timestamp}`}
                    type="button"
                    data-testid={`query-history-${entry.query.slice(0, 20)}`}
                    onClick={() => handleHistoryClick(entry)}
                    style={{
                      display: 'block',
                      width: '100%',
                      padding: '4px var(--space-2)',
                      background: 'transparent',
                      border: 'none',
                      borderRadius: 'var(--radius-sm)',
                      cursor: 'pointer',
                      textAlign: 'left',
                      fontSize: 'var(--font-size-caption)',
                      color: 'var(--color-text-secondary)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                    onMouseEnter={e => {
                      e.currentTarget.style.background =
                        'var(--color-surface-subtle)'
                    }}
                    onMouseLeave={e => {
                      e.currentTarget.style.background = 'transparent'
                    }}
                    title={entry.query}
                  >
                    {entry.query}
                  </button>
                ))}
              </div>
            )}

            {/* Snippets */}
            <QuerySnippets onInsert={handleSnippetInsert} />
          </div>
        )}

        {/* ─── Results ─── */}
        <div
          data-testid="query-builder-results"
          style={{ flex: 1, minWidth: 0 }}
        >
          {state.viewMode === 'results' ? (
            <QueryResults
              result={result}
              loading={loading}
            />
          ) : (
            <TableView
              columns={columns}
              rows={result?.results ?? []}
              onSort={handleSort}
            />
          )}
        </div>
      </div>

      {/* ─── View mode toggle ─── */}
      <div
        style={{
          display: 'flex',
          gap: 'var(--space-2)',
          justifyContent: 'flex-end',
        }}
      >
        <button
          type="button"
          data-testid="query-view-toggle"
          onClick={() =>
            setState(prev => ({
              ...prev,
              viewMode: prev.viewMode === 'results' ? 'table' : 'results',
            }))
          }
          style={{
            padding: '4px var(--space-2)',
            background: 'transparent',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--color-text-muted)',
            fontSize: 'var(--font-size-caption)',
            cursor: 'pointer',
          }}
        >
          {state.viewMode === 'results'
            ? 'Table view'
            : 'Results view'}
        </button>
      </div>
    </div>
  )
}

// ─── Chips → DSL conversion ─────────────────────────────────────

function chipsToDsl(chips: FilterChip[]): string {
  const parts = chips.map(chip => {
    if (!chip.key) return ''
    if (chip.value2 !== undefined) {
      return `(property "${chip.key}" "${chip.op}" "${chip.value}" "${chip.value2}")`
    }
    return `(property "${chip.key}" "${chip.value}")`
  })
  return parts.filter(Boolean).join(' ')
}
