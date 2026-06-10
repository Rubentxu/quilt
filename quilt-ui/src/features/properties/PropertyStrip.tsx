/**
 * PropertyStrip — compact card displaying multiple block properties
 * in a structured layout below the block content.
 *
 * Inspired by Logseq's property drawer and Notion's inline property strip:
 *   - Each property on its own row with key label + styled value
 *   - Color-coded by property key type (status=green, priority=badge, etc.)
 *   - Clickable for inline editing via api.setBlockProperty
 *   - Subtle background card with rounded corners
 */
import { useState, useCallback } from 'react'
import {
  Calendar, Clock, User, FileText, Tag, Hash,
  CheckCircle2, AlertCircle, type LucideIcon,
} from 'lucide-react'
import { api } from '@core/api-client'
import toast from 'react-hot-toast'
import type { Block } from '@shared/types/api'

// ── Types ───────────────────────────────────────────────────────────

export interface PropertyRow {
  key: string
  value: string
}

interface PropertyStripProps {
  block: Block
  properties: PropertyRow[]
  /** Called when a property is updated via the strip */
  onUpdate?: (block: Block) => void
}

// ── Style helpers ───────────────────────────────────────────────────

interface PropertyStyle {
  icon?: LucideIcon
  iconColor?: string
  valueColor?: string
  valueBg?: string
  badge?: boolean
}

const PROPERTY_META: Record<string, PropertyStyle> = {
  status:     { icon: CheckCircle2, iconColor: 'var(--color-success)', valueColor: 'var(--color-success)', bg: 'var(--color-success-subtle)', badge: true },
  priority:   { icon: AlertCircle, iconColor: 'var(--color-warning)', badge: true },
  deadline:   { icon: Calendar },
  scheduled:  { icon: Clock },
  'created-by': { icon: User },
  created_by: { icon: User },
  template:   { icon: FileText },
  tags:       { icon: Tag },
  type:       { icon: Hash },
}

function getPropertyStyle(key: string): PropertyStyle {
  return PROPERTY_META[key.toLowerCase()] ?? {}
}

function formatValue(key: string, value: string): string {
  if (key === 'priority') return `P${value.toUpperCase()}`
  if (key === 'status') return value.toUpperCase()
  return value
}

// ── Component ───────────────────────────────────────────────────────

export function PropertyStrip({ block, properties, onUpdate }: PropertyStripProps) {
  if (!properties.length) return null

  return (
    <div
      data-testid="property-strip"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '3px',
        marginTop: '6px',
        padding: '6px 10px',
        borderRadius: 'var(--radius-md, 6px)',
        background: 'var(--color-surface-subtle, rgba(128,128,128,0.06))',
        border: '1px solid var(--color-border-subtle, rgba(128,128,128,0.15))',
        flexShrink: 0,
        maxWidth: '420px',
        animation: 'fadeIn 0.15s ease',
      }}
    >
      {properties.map((prop) => {
        const meta = getPropertyStyle(prop.key)
        const Icon = meta.icon
        const displayValue = formatValue(prop.key, prop.value)
        const valueStyle = meta.valueColor
          ? { color: meta.valueColor }
          : {}
        const badgeStyle = meta.valueColor
          ? { background: `${meta.valueColor}18`, borderRadius: '4px', padding: '1px 6px' }
          : {}

        return (
          <PropertyRowItem
            key={prop.key}
            block={block}
            propKey={prop.key}
            value={prop.value}
            displayValue={displayValue}
            Icon={Icon}
            iconColor={meta.iconColor}
            valueStyle={valueStyle}
            badgeStyle={badgeStyle}
            onUpdate={onUpdate}
          />
        )
      })}

      {/* Fade-in keyframes injected once */}
      <style>{`
        @keyframes fadeIn {
          from { opacity: 0; transform: translateY(-2px); }
          to   { opacity: 1; transform: translateY(0); }
        }
      `}</style>
    </div>
  )
}

// ── Single property row ─────────────────────────────────────────────

function PropertyRowItem({
  block, propKey, value, displayValue,
  Icon, iconColor, valueStyle, badgeStyle, onUpdate,
}: {
  block: Block
  propKey: string
  value: string
  displayValue: string
  Icon?: LucideIcon
  iconColor?: string
  valueStyle: React.CSSProperties
  badgeStyle: React.CSSProperties
  onUpdate?: (block: Block) => void
}) {
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState(value)

  const handleSave = useCallback(async () => {
    const trimmed = editValue.trim()
    if (trimmed === value) { setEditing(false); return }
    try {
      await api.setBlockProperty(block.id, propKey, trimmed)
      setEditing(false)
      // Update the local block reference
      if (onUpdate) {
        onUpdate({ ...block, properties: [...(block.properties ?? []).filter(p => p.key !== propKey), { key: propKey, value: trimmed }] })
      }
    } catch {
      toast.error(`Failed to update ${propKey}`)
    }
  }, [block, propKey, editValue, value, onUpdate])

  const handleKey = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSave()
    if (e.key === 'Escape') { setEditValue(value); setEditing(false) }
  }, [handleSave, value])

  return (
    <div
      data-testid={`property-strip-row-${propKey}`}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        padding: '3px 4px',
        borderRadius: '4px',
        transition: 'background 0.1s',
        cursor: editing ? 'default' : 'pointer',
      }}
      onClick={() => { if (!editing) { setEditValue(value); setEditing(true) } }}
    >
      {/* Key label — subtle, monospace-like */}
      <span
        style={{
          minWidth: '64px',
          fontSize: '11px',
          fontWeight: 500,
          color: 'var(--color-text-muted)',
          fontFamily: 'var(--font-mono, monospace)',
          textTransform: 'lowercase',
          letterSpacing: '0.02em',
          display: 'flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        {Icon && <Icon size={12} color={iconColor} />}
        {propKey}
      </span>

      {/* Separator dot */}
      <span style={{ color: 'var(--color-border-strong)', fontSize: '8px' }}>●</span>

      {/* Value — prominent, editable on click */}
      {editing ? (
        <input
          autoFocus
          data-testid={`property-strip-input-${propKey}`}
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onBlur={handleSave}
          onKeyDown={handleKey}
          style={{
            border: 'none',
            background: 'var(--color-surface)',
            color: 'var(--color-text)',
            fontSize: '12px',
            fontWeight: 500,
            padding: '2px 6px',
            borderRadius: '4px',
            width: '120px',
            outline: '1px solid var(--color-primary)',
          }}
        />
      ) : (
        <span
          style={{
            fontSize: '12px',
            fontWeight: 600,
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            ...valueStyle,
            ...badgeStyle,
          }}
        >
          {displayValue || <span style={{ color: 'var(--color-text-disabled)', fontStyle: 'italic' }}>empty</span>}
        </span>
      )}
    </div>
  )
}
