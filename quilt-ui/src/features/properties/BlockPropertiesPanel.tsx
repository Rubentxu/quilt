import { useState, useEffect, useCallback } from 'react'
import { Plus, X, Tag, Hash, ToggleLeft, Calendar, Link2 } from 'lucide-react'
import { api } from '@core/api-client'
import type { BlockProperty } from '@shared/types/api'
import toast from 'react-hot-toast'

interface BlockPropertiesPanelProps {
  blockId: string
  onClose: () => void
}

const PROPERTY_TYPE_ICONS: Record<string, React.ReactNode> = {
  string: <Tag size={14} />,
  number: <Hash size={14} />,
  boolean: <ToggleLeft size={14} />,
  date: <Calendar size={14} />,
  select: <Tag size={14} />,
  page_ref: <Link2 size={14} />,
}

export function BlockPropertiesPanel({ blockId, onClose }: BlockPropertiesPanelProps) {
  const [properties, setProperties] = useState<BlockProperty[]>([])
  const [loading, setLoading] = useState(true)
  const [newKey, setNewKey] = useState('')
  const [showAddForm, setShowAddForm] = useState(false)

  useEffect(() => {
    loadProperties()
  }, [blockId])

  async function loadProperties() {
    try {
      const props = await api.getBlockProperties(blockId)
      setProperties(props)
    } catch {
      // Properties endpoint may not exist yet on the backend
      setProperties([])
    } finally {
      setLoading(false)
    }
  }

  async function updateProperty(key: string, value: unknown) {
    try {
      await api.setBlockProperty(blockId, key, value)
      setProperties(prev =>
        prev.map(p =>
          p.key === key
            ? { ...p, value: value as string | number | boolean | null }
            : p,
        ),
      )
    } catch {
      toast.error('Failed to update property')
    }
  }

  async function deleteProperty(key: string) {
    try {
      await api.deleteBlockProperty(blockId, key)
      setProperties(prev => prev.filter(p => p.key !== key))
    } catch {
      toast.error('Failed to delete property')
    }
  }

  async function addProperty() {
    if (!newKey.trim()) return
    try {
      await api.setBlockProperty(blockId, newKey.trim(), '')
      setNewKey('')
      setShowAddForm(false)
      loadProperties()
    } catch {
      toast.error('Failed to add property')
    }
  }

  if (loading) {
    return (
      <div
        style={{
          padding: 'var(--space-4)',
          color: 'var(--color-text-muted)',
          fontSize: '13px',
        }}
      >
        Loading properties...
      </div>
    )
  }

  return (
    <div
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: 'var(--space-3) var(--space-4)',
          borderBottom: '1px solid var(--color-border)',
        }}
      >
        <span
          style={{
            fontSize: '13px',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
          }}
        >
          Properties
        </span>
        <button
          onClick={() => setShowAddForm(true)}
          aria-label="Add property"
          title="Add property"
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--color-text-muted)',
            padding: '2px',
            display: 'flex',
            alignItems: 'center',
          }}
        >
          <Plus size={16} />
        </button>
      </div>

      {/* Properties list */}
      {properties.length === 0 && !showAddForm ? (
        <div
          style={{
            padding: 'var(--space-6) var(--space-4)',
            textAlign: 'center',
            color: 'var(--color-text-muted)',
            fontSize: '13px',
          }}
        >
          No properties yet
        </div>
      ) : (
        <div>
          {properties.map(prop => (
            <div
              key={prop.key}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 'var(--space-2)',
                padding: 'var(--space-2) var(--space-4)',
                borderBottom: '1px solid var(--color-border)',
                fontSize: '13px',
              }}
            >
              <span
                style={{
                  color: 'var(--color-text-muted)',
                  display: 'flex',
                  alignItems: 'center',
                }}
              >
                {PROPERTY_TYPE_ICONS[prop.type] || <Tag size={14} />}
              </span>
              <span
                style={{
                  color: 'var(--color-text-secondary)',
                  minWidth: '100px',
                  fontWeight: 500,
                }}
              >
                {prop.key}
              </span>
              {prop.type === 'boolean' ? (
                <input
                  type="checkbox"
                  checked={!!prop.value}
                  onChange={e => updateProperty(prop.key, e.target.checked)}
                  style={{ cursor: 'pointer' }}
                />
              ) : (
                <input
                  type={prop.type === 'number' ? 'number' : 'text'}
                  value={String(prop.value ?? '')}
                  onChange={e => {
                    const val =
                      prop.type === 'number'
                        ? Number(e.target.value)
                        : e.target.value
                    updateProperty(prop.key, val)
                  }}
                  style={{
                    flex: 1,
                    border: 'none',
                    outline: 'none',
                    background: 'transparent',
                    color: 'var(--color-text-primary)',
                    fontSize: '13px',
                    padding: '2px 0',
                    fontFamily: 'inherit',
                  }}
                />
              )}
              <button
                onClick={() => deleteProperty(prop.key)}
                aria-label={`Delete property ${prop.key}`}
                title={`Delete ${prop.key}`}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  color: 'var(--color-text-disabled)',
                  padding: '2px',
                  display: 'flex',
                  alignItems: 'center',
                  opacity: 0.5,
                  transition: 'opacity var(--motion-fast)',
                }}
                onMouseEnter={e => {
                  (e.currentTarget as HTMLButtonElement).style.opacity = '1'
                }}
                onMouseLeave={e => {
                  (e.currentTarget as HTMLButtonElement).style.opacity = '0.5'
                }}
              >
                <X size={12} />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Add property form */}
      {showAddForm && (
        <div
          style={{
            display: 'flex',
            gap: 'var(--space-2)',
            padding: 'var(--space-3) var(--space-4)',
            borderTop: '1px solid var(--color-border)',
          }}
        >
          <input
            type="text"
            value={newKey}
            onChange={e => setNewKey(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') addProperty()
              if (e.key === 'Escape') {
                setShowAddForm(false)
                setNewKey('')
              }
            }}
            placeholder="Property name"
            autoFocus
            style={{
              flex: 1,
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: 'var(--space-1) var(--space-2)',
              fontSize: '13px',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              outline: 'none',
              fontFamily: 'inherit',
            }}
          />
          <button
            onClick={addProperty}
            style={{
              padding: 'var(--space-1) var(--space-3)',
              background: 'var(--color-accent)',
              color: 'white',
              border: 'none',
              borderRadius: 'var(--radius-sm)',
              fontSize: '13px',
              cursor: 'pointer',
            }}
          >
            Add
          </button>
        </div>
      )}
    </div>
  )
}
