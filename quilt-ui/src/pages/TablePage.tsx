/**
 * TablePage — F17 Query Builder + TableView integration.
 *
 * Exposes the existing QueryBuilder (filter chips + virtualized table)
 * as a standalone route at `/table`. The page fetches available
 * property keys on mount so the filter chip dropdown is populated.
 *
 * This is a thin wrapper — all the real logic lives in:
 *   - features/query-builder/QueryBuilder.tsx (orchestration)
 *   - features/table-view/TableView.tsx (virtualized rendering)
 *   - features/filter-chips/FilterChipGroup.tsx (chip UI)
 *   - shared/utils/buildQueryAst.ts (DSL construction)
 */

import { useEffect, useState } from 'react'
import { toast } from 'react-hot-toast'
import { api } from '@core/api-client'
import { QueryBuilder } from '@features/query-builder/QueryBuilder'
import type { ColumnDef } from '@features/table-view/ColumnDef'

/** Default columns for the table view. */
const DEFAULT_COLUMNS: ColumnDef[] = [
  { key: 'name', header: 'Name', width: 200, sortable: true },
  { key: 'content', header: 'Content', width: 300, sortable: false },
  { key: 'block_type', header: 'Type', width: 100, sortable: true },
  { key: 'created_at', header: 'Created', width: 140, sortable: true },
  { key: 'updated_at', header: 'Updated', width: 140, sortable: true },
]

export function TablePage() {
  const [availableKeys, setAvailableKeys] = useState<string[]>([])

  // Fetch property keys for the filter chip dropdown
  useEffect(() => {
    // Property keys come from the cross-block aggregation endpoint
    // (`GET /api/v1/properties/keys`), not from a per-block
    // properties call. The previous `getBlockProperties('')` hack
    // sent `''` as a block ID and 404'd.
    api
      .listPropertyKeys()
      .then(({ keys }) => {
        setAvailableKeys([...keys].sort())
      })
      .catch(() => {
        // Non-fatal — filter dropdown will be empty
      })
  }, [])

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
          Table View
        </h1>
        <p
          style={{
            fontSize: '13px',
            color: 'var(--color-text-muted)',
            margin: 0,
          }}
        >
          Filter, sort, and query your knowledge graph
        </p>
      </div>

      <QueryBuilder
        columns={DEFAULT_COLUMNS}
        availableKeys={availableKeys}
        defaultLimit={100}
        onResults={(result) => {
          // Optional: show elapsed time
          if (result.elapsed_ms > 1000) {
            toast(`Query took ${result.elapsed_ms}ms`, { icon: '⏱️' })
          }
        }}
        onError={(error) => {
          toast.error(`Query failed: ${error.message}`)
        }}
      />
    </div>
  )
}
