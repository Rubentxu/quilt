/**
 * Badge / Tag — DESIGN.md §9.6
 *
 * Compact, pill-shaped metadata indicator.
 * Variants: default, success, warning, danger, info
 *
 * Apariencia: background primary-container sutil, color primary, pill 999px,
 * 12px / 600 weight, 2px 8px padding.
 */

import { type ReactNode } from 'react'

export type BadgeVariant = 'default' | 'success' | 'warning' | 'danger' | 'info'

interface BadgeProps {
  children: ReactNode
  variant?: BadgeVariant
  title?: string
}

const VARIANT_STYLES: Record<BadgeVariant, { bg: string; fg: string }> = {
  default: { bg: 'var(--color-primary-container)', fg: 'var(--color-primary)' },
  success: { bg: 'var(--color-success-subtle)', fg: 'var(--color-success)' },
  warning: { bg: 'var(--color-warning-subtle)', fg: 'var(--color-warning)' },
  danger: { bg: 'var(--color-danger-subtle)', fg: 'var(--color-danger)' },
  info: { bg: 'var(--color-info-subtle)', fg: 'var(--color-info)' },
}

export function Badge({ children, variant = 'default', title }: BadgeProps) {
  const v = VARIANT_STYLES[variant]
  return (
    <span
      title={title}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 'var(--space-1)',
        borderRadius: 'var(--radius-pill)',
        background: v.bg,
        color: v.fg,
        fontSize: '12px',
        fontWeight: 600,
        lineHeight: 1,
        padding: '2px 8px',
        whiteSpace: 'nowrap',
        maxWidth: '100%',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
      }}
    >
      {children}
    </span>
  )
}
