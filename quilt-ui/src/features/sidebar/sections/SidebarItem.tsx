// ─── SidebarItem ─────────────────────────────────────────────
// States per DESIGN.md §9.1:
//   Default: text-secondary, no background
//   Hover:   bg-surface-subtle
//   Active:  bg-primary-container, text-primary, **left border** (NOT colour-only)
//   Focus:   ring-primary
// §9.1 explicitly requires the active state to be evident without
// colour alone — the left border is rendered as a 3px-wide absolutely
// positioned span so it doesn't shift the rest of the row's padding.

import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'

interface SidebarItemProps {
  icon: ReactNode
  label: string
  href: string
  active?: boolean
  collapsed?: boolean
  dataTestId?: string
}

export function SidebarItem({
  icon,
  label,
  href,
  active,
  collapsed,
  dataTestId,
}: SidebarItemProps) {
  return (
    <Link
      to={href as any}
      title={collapsed ? label : undefined}
      data-testid={dataTestId}
      data-active={active ? 'true' : undefined}
      aria-current={active ? 'page' : undefined}
      style={{
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-2)',
        padding: '10px var(--space-3)',
        paddingLeft: collapsed ? 'var(--space-2)' : 'calc(var(--space-2) + 3px)',
        borderRadius: '12px',
        textDecoration: 'none',
        fontSize: '13px',
        fontWeight: active ? 600 : 400,
        color: active ? 'var(--color-primary)' : 'var(--color-text-secondary)',
        background: active ? 'var(--color-primary-container)' : 'transparent',
        minHeight: '40px',
        transition:
          'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
        overflow: 'hidden',
        whiteSpace: 'nowrap',
        textOverflow: 'ellipsis',
      }}
      className="sidebar-item"
    >
      {/* Active indicator — visible border on the left edge
          (§9.1: must not depend on colour only) */}
      {active && (
        <span
          aria-hidden="true"
          style={{
            position: 'absolute',
            left: 0,
            top: '4px',
            bottom: '4px',
            width: '3px',
            borderRadius: 'var(--radius-pill)',
            background: 'var(--color-primary)',
          }}
        />
      )}
      <span style={{ flexShrink: 0, display: 'flex', alignItems: 'center' }}>{icon}</span>
      {!collapsed && (
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{label}</span>
      )}
    </Link>
  )
}
