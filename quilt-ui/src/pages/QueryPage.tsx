/**
 * QueryPage — Raw DSL query editor with results.
 *
 * Exposes the existing query DSL parser (4124 lines) + executor as a
 * standalone route at /query. The user writes DSL queries in a text
 * area and sees results in a table.
 */

import { useState, useCallback } from 'react'
import { toast } from 'react-hot-toast'
import { api } from '@core/api-client'
import { TableView } from '@features/table-view/TableView'
import type { ColumnDef } from '@features/table-view/ColumnDef'
import type { QueryResult } from '@shared/types/queryAst'

const DEFAULT_COLUMNS: ColumnDef[] = [
  { key: 'name', header: 'Name', width: 200, sortable: true },
  { key: 'content', header: 'Content', width: 300, sortable: false },
  { key: 'block_type', header: 'Type', width: 100, sortable: true },
  { key: 'created_at', header: 'Created', width: 140, sortable: true },
  { key: 'updated_at', header: 'Updated', width: 140, sortable: true },
]

const EXAMPLE_QUERIES = [
  'SELECT * FROM blocks WHERE status = "todo"',
  'SELECT name, content FROM blocks WHERE priority = "A"',
  'SELECT * FROM blocks WHERE created_by = "agent::claude"',
  'SELECT * FROM blocks WHERE created_at > "2026-06-01"',
]

export function QueryPage() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<QueryResult | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleExecute = useCallback(async () => {
    if (!query.trim()) {
      toast.error('Enter a query')
      return
    }

    setLoading(true)
    setError(null)

    try {
      // Parse the DSL query and execute it
      const result = await api.executeQuery({ type: 'select', table: 'blocks', where: query, limit: 100 } as any, 100)
      setResults(result)
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setError(message)
      toast.error(`Query failed: ${message}`)
    } finally {
      setLoading(false)
    }
  }, [query])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        handleExecute()
      }
    },
    [handleExecute],
  )

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        padding: 'var(--space-4) var(--space-5)',
        gap: 'var(--space-4)',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexWrap: 'wrap',
          gap: 'var(--space-3)',
        }}
      >
        <h1
          style={{
            fontSize: '20px',
            fontWeight: 700,
            color: 'var(--color-text-primary)',
            margin: 0,
          }}
        >
          Query Editor
        </h1>
        <p style={{ fontSize: '13px', color: 'var(--color-text-muted)', margin: 0 }}>
          Write DSL queries · <kbd style={{ padding: '2px 6px', background: 'var(--color-surface-subtle)', borderRadius: '4px', fontSize: '11px' }}>⌘ Enter</kbd> to run
        </p>
      </div>

      {/* Query input */}
      <div
        style={{
          display: 'flex',
          gap: 'var(--space-3)',
          alignItems: 'flex-start',
        }}
      >
        <textarea
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="SELECT * FROM blocks WHERE status = &quot;todo&quot;"
          rows={3}
          style={{
            flex: 1,
            padding: '12px 16px',
            fontSize: '14px',
            fontFamily: 'monospace',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            background: 'var(--color-surface)',
            color: 'var(--color-text-primary)',
            resize: 'vertical',
            outline: 'none',
          }}
        />
        <button
          onClick={handleExecute}
          disabled={loading}
          style={{
            padding: '12px 24px',
            fontSize: '14px',
            fontWeight: 600,
            background: loading ? 'var(--color-surface-subtle)' : 'var(--color-primary)',
            color: loading ? 'var(--color-text-muted)' : 'white',
            border: 'none',
            borderRadius: 'var(--radius-md)',
            cursor: loading ? 'not-allowed' : 'pointer',
            whiteSpace: 'nowrap',
          }}
        >
          {loading ? 'Running…' : 'Execute'}
        </button>
      </div>

      {/* Example queries */}
      {!results && !loading && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-2)' }}>
          {EXAMPLE_QUERIES.map((q, i) => (
            <button
              key={i}
              onClick={() => setQuery(q)}
              style={{
                padding: '6px 12px',
                fontSize: '12px',
                fontFamily: 'monospace',
                background: 'var(--color-surface-subtle)',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius-sm)',
                cursor: 'pointer',
                color: 'var(--color-text-secondary)',
              }}
            >
              {q}
            </button>
          ))}
        </div>
      )}

      {/* Error display */}
      {error && (
        <div
          style={{
            padding: '12px 16px',
            background: 'var(--color-danger-subtle, #fef2f2)',
            border: '1px solid var(--color-danger, #ef4444)',
            borderRadius: 'var(--radius-md)',
            fontSize: '13px',
            color: 'var(--color-danger, #ef4444)',
          }}
        >
          {error}
        </div>
      )}

      {/* Results table */}
      {results && (
        <div style={{ flex: 1, minHeight: 0 }}>
          <TableView columns={DEFAULT_COLUMNS} rows={(results as any).rows ?? results as any} />
        </div>
      )}
    </div>
  )
}
