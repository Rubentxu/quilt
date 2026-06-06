/**
 * KanbanPage — Kanban board view for blocks grouped by property.
 *
 * Exposes the existing KanbanBoard (430 LOC with drag & drop) as a
 * standalone route at /kanban. The user picks a property key (status,
 * priority, etc.) and the board groups blocks into columns by that
 * property's value.
 */

import { useEffect, useState, useCallback } from 'react'
import { toast } from 'react-hot-toast'
import { api } from '@core/api-client'
import { KanbanBoard } from '@features/kanban/KanbanBoard'
import type { Block } from '@shared/types/api'

export function KanbanPage() {
  const [blocks, setBlocks] = useState<Block[]>([])
  const [propertyKey, setPropertyKey] = useState('status')
  const [availableKeys, setAvailableKeys] = useState<string[]>([])
  const [loading, setLoading] = useState(true)

  // Fetch blocks + available property keys on mount
  useEffect(() => {
    setLoading(true)
    Promise.all([
      api.listPages().then(pages =>
        Promise.all(
          pages
            .filter(p => !p.journal)
            .slice(0, 50)
            .map(p => api.getPageBlocks(p.name))
        )
      ).then(blockArrays => blockArrays.flat()),
      api.getBlockProperties('').then(props =>
        [...new Set(props.map((p: { key: string }) => p.key))].sort()
      ),
    ])
      .then(([allBlocks, keys]) => {
        setBlocks(allBlocks)
        setAvailableKeys(keys.filter((k: string) => k !== 'card-shape' && k !== 'icon'))
      })
      .catch(() => toast.error('Failed to load blocks'))
      .finally(() => setLoading(false))
  }, [])

  const handlePropertyChange = useCallback(
    (blockId: string, key: string, value: string) => {
      api.setBlockProperty(blockId, key, value)
        .then(() => {
          setBlocks(prev =>
            prev.map(b => {
              if (b.id !== blockId) return b
              const existing = b.properties ?? []
              const idx = existing.findIndex(p => p.key === key)
              const updated = [...existing]
              if (idx >= 0) {
                updated[idx] = { ...updated[idx], value }
              } else {
                updated.push({ key, value, type: 'string' })
              }
              return { ...b, properties: updated }
            })
          )
        })
        .catch(() => toast.error('Failed to update property'))
    },
    [],
  )

  if (loading) {
    return (
      <div style={{ padding: 'var(--space-6)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
        Loading kanban board…
      </div>
    )
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 'var(--space-4) var(--space-5)', gap: 'var(--space-4)' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 'var(--space-3)' }}>
        <h1 style={{ fontSize: '20px', fontWeight: 700, color: 'var(--color-text-primary)', margin: 0 }}>
          Kanban Board
        </h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <label style={{ fontSize: '13px', color: 'var(--color-text-muted)' }}>Group by:</label>
          <select
            value={propertyKey}
            onChange={e => setPropertyKey(e.target.value)}
            style={{
              padding: '6px 12px',
              fontSize: '13px',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-md)',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              cursor: 'pointer',
            }}
          >
            {availableKeys.map(k => (
              <option key={k} value={k}>{k}</option>
            ))}
          </select>
        </div>
      </div>

      {blocks.length === 0 ? (
        <div style={{ padding: 'var(--space-8)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
          No blocks with properties found. Add properties to blocks to use the kanban view.
        </div>
      ) : (
        <KanbanBoard
          propertyKey={propertyKey}
          blocks={blocks}
          onPropertyChange={handlePropertyChange}
        />
      )}
    </div>
  )
}
