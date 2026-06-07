// ─── LayoutMenu — dashboard layout dropdown ──────────────────────
//
// The user-facing surface of the dashboard feature. Renders a
// single button in the top bar that opens a dropdown with:
//   - Three preset buttons (Default / Focus / Review).
//   - One checkbox per known panel (Sidebar / Backlinks /
//     Agent activity / Outline).
//
// The component is purely presentational — it reads and writes the
// PanelVisibilityContext. It does not touch the router, the
// CommandRegistry, or localStorage directly.

import { useEffect, useRef, useState } from 'react'
import { LayoutGrid, Check } from 'lucide-react'
import { usePanelVisibility } from './PanelVisibilityContext'
import {
  DEFAULT_PANELS,
  PANEL_LABELS,
} from './PanelVisibilityContext'
import {
  PRESET_LABELS,
  PRESET_ORDER,
  type PanelId,
  type PresetId,
} from './presets'

export function LayoutMenu() {
  const [open, setOpen] = useState(false)
  const wrapperRef = useRef<HTMLDivElement>(null)
  const {
    visiblePanels,
    togglePanel,
    applyPreset,
    lastAppliedPreset,
  } = usePanelVisibility()

  // Close on outside click and Escape — matches the TopbarMenu
  // pattern from AppShell. Keeping the dismiss behaviour
  // consistent means users only have to learn it once.
  useEffect(() => {
    if (!open) return
    function handleClickOutside(e: MouseEvent) {
      const target = e.target as Node
      if (wrapperRef.current && !wrapperRef.current.contains(target)) {
        setOpen(false)
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleKey)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleKey)
    }
  }, [open])

  return (
    <div ref={wrapperRef} style={{ position: 'relative' }}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-label="Layout"
        title="Layout"
        aria-haspopup="menu"
        aria-expanded={open}
        data-testid="layout-menu-trigger"
        className="topbar-action"
        style={triggerStyle}
      >
        <LayoutGrid size={17} aria-hidden="true" />
      </button>
      {open && (
        <div
          role="menu"
          aria-label="Layout"
          data-testid="layout-menu"
          style={menuStyle}
        >
          <div style={sectionLabelStyle}>Presets</div>
          <div style={presetRowStyle}>
            {PRESET_ORDER.map((id) => (
              <PresetButton
                key={id}
                id={id}
                active={lastAppliedPreset === id}
                onClick={() => {
                  applyPreset(id)
                  setOpen(false)
                }}
              />
            ))}
          </div>

          <div
            style={{
              height: 1,
              background: 'var(--color-border)',
              margin: 'var(--space-1) 0',
            }}
          />

          <div style={sectionLabelStyle}>Panels</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            {DEFAULT_PANELS.map((id) => (
              <PanelToggle
                key={id}
                id={id}
                checked={visiblePanels.has(id)}
                onToggle={() => togglePanel(id)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

interface PresetButtonProps {
  id: PresetId
  active: boolean
  onClick: () => void
}

function PresetButton({ id, active, onClick }: PresetButtonProps) {
  return (
    <button
      type="button"
      role="menuitem"
      data-testid={`layout-preset-${id}`}
      onClick={onClick}
      aria-pressed={active}
      style={{
        ...presetButtonStyle,
        background: active ? 'var(--color-accent-subtle, rgba(99, 102, 241, 0.10))' : 'transparent',
        color: active ? 'var(--color-accent)' : 'var(--color-text-primary)',
      }}
    >
      {PRESET_LABELS[id]}
    </button>
  )
}

interface PanelToggleProps {
  id: PanelId
  checked: boolean
  onToggle: () => void
}

function PanelToggle({ id, checked, onToggle }: PanelToggleProps) {
  return (
    <label
      data-testid={`layout-toggle-${id}`}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-2)',
        padding: 'var(--space-1) var(--space-2)',
        borderRadius: 'var(--radius-sm)',
        cursor: 'pointer',
        fontSize: '13px',
        color: 'var(--color-text-primary)',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.background = 'var(--color-surface-subtle)'
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = 'transparent'
      }}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={onToggle}
        aria-label={PANEL_LABELS[id]}
        style={{ cursor: 'pointer' }}
      />
      <span style={{ flex: 1 }}>{PANEL_LABELS[id]}</span>
      {checked && (
        <Check
          size={14}
          aria-hidden="true"
          style={{ color: 'var(--color-accent)' }}
        />
      )}
    </label>
  )
}

// ─── styles ───────────────────────────────────────────────────────
//
// Kept inline (matching the TopbarMenu pattern from AppShell) so
// the component does not need a sibling CSS file. Every value
// resolves to a `--color-*` / `--space-*` design-system token, so
// light/dark mode and density changes propagate automatically.

const triggerStyle = {
  width: '32px',
  height: '32px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  cursor: 'pointer',
  background: 'none',
  border: 'none',
  color: 'var(--color-text-secondary)',
  borderRadius: 'var(--radius-md)',
  padding: 0,
} as const

const menuStyle = {
  position: 'absolute',
  top: 'calc(100% + 4px)',
  right: 0,
  minWidth: '220px',
  background: 'var(--color-surface-elevated)',
  border: '1px solid var(--color-border)',
  borderRadius: 'var(--radius-md)',
  boxShadow: 'var(--shadow-md)',
  padding: 'var(--space-2)',
  zIndex: 100,
  display: 'flex',
  flexDirection: 'column',
  gap: 'var(--space-1)',
} as const

const sectionLabelStyle = {
  fontSize: '11px',
  fontWeight: 600,
  textTransform: 'uppercase' as const,
  letterSpacing: '0.04em',
  color: 'var(--color-text-muted)',
  padding: 'var(--space-1) var(--space-2)',
} as const

const presetRowStyle = {
  display: 'flex',
  gap: 'var(--space-1)',
} as const

const presetButtonStyle = {
  flex: 1,
  padding: 'var(--space-1) var(--space-2)',
  border: '1px solid var(--color-border)',
  borderRadius: 'var(--radius-sm)',
  fontSize: '12px',
  fontWeight: 500,
  fontFamily: 'inherit',
  cursor: 'pointer',
  transition:
    'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
} as const
