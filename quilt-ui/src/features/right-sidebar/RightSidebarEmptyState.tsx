// ─── RightSidebarEmptyState ────────────────────────────────────────────────
//
// Shown when no section is visible or the sidebar is empty.
// Per GS-8 spec: "empty state shown, suppressed when content exists,
// transition without flash."

export function RightSidebarEmptyState() {
  return (
    <div
      data-testid="right-sidebar-empty"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 'var(--space-8) var(--space-4)',
        textAlign: 'center',
        gap: 'var(--space-3)',
      }}
    >
      <div
        style={{
          width: '40px',
          height: '40px',
          borderRadius: '50%',
          background: 'var(--color-surface-subtle)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--color-text-muted)',
          fontSize: '18px',
        }}
      >
        ⊙
      </div>
      <div>
        <div
          style={{
            fontSize: '13px',
            color: 'var(--color-text-secondary)',
            fontWeight: 500,
            marginBottom: 'var(--space-1)',
          }}
        >
          Nothing selected
        </div>
        <div
          style={{
            fontSize: '12px',
            color: 'var(--color-text-muted)',
          }}
        >
          Select a block or page to see contextual actions
        </div>
      </div>
    </div>
  )
}
