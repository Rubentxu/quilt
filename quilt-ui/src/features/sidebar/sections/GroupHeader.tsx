// ─── Group header ─────────────────────────────────────────────
// Per DESIGN.md §9.1, sidebar section headers are uppercase
// muted-style h3 elements that disappear when the sidebar collapses.
// This is a leaf presentation component — no state, no hooks.

interface GroupHeaderProps {
  label: string
  collapsed?: boolean
}

export function GroupHeader({ label, collapsed }: GroupHeaderProps) {
  if (collapsed) return null
  return (
    <h3
      style={{
        fontSize: '11px',
        fontWeight: 600,
        textTransform: 'uppercase' as const,
        letterSpacing: '0.05em',
        color: 'var(--color-text-muted)',
        padding: '0 var(--space-3)',
        marginBottom: 'var(--space-2)',
      }}
    >
      {label}
    </h3>
  )
}
