// ─── SidebarSkeleton ─────────────────────────────────────────────
// Loading placeholder shown by the sidebar while `api.listPages()`
// is in flight. Six stacked rows with a pulsing animation give a
// stable visual rhythm that matches the eventual page list density.

export function SidebarSkeleton() {
  return (
    <div
      data-testid="sidebar-skeleton"
      style={{
        padding: 'var(--space-3)',
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-3)',
      }}
    >
      {Array.from({ length: 6 }).map((_, i) => (
        <div
          key={i}
          style={{
            height: '16px',
            width: `${60 + Math.random() * 30}%`,
            background: 'var(--color-surface-subtle)',
            borderRadius: 'var(--radius-sm)',
            animation: 'pulse 1.5s ease-in-out infinite',
          }}
        />
      ))}
    </div>
  )
}
