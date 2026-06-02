import type { CSSProperties } from 'react'

interface SkeletonProps {
  width?: string | number
  height?: string | number
  borderRadius?: string
  style?: CSSProperties
}

export function Skeleton({ width = '100%', height = '16px', borderRadius = 'var(--radius-sm)', style }: SkeletonProps) {
  return (
    <div
      style={{
        width,
        height,
        borderRadius,
        background: 'var(--color-surface-subtle)',
        animation: 'pulse 1.5s ease-in-out infinite',
        ...style,
      }}
    />
  )
}

/** Multiple skeleton lines like a loading paragraph */
export function SkeletonLines({ count = 3 }: { count?: number }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)', padding: 'var(--space-6) var(--space-4)' }}>
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} style={{ display: 'flex', gap: 'var(--space-2)' }}>
          <Skeleton width="8px" height="8px" borderRadius="50%" style={{ marginTop: '4px' }} />
          <Skeleton width={`${30 + Math.random() * 50}%`} />
        </div>
      ))}
    </div>
  )
}

/** Page loading skeleton with header and content */
export function PageSkeleton() {
  return (
    <div>
      <div style={{ padding: 'var(--space-6) var(--space-4) var(--space-4)' }}>
        <Skeleton width="40%" height="28px" />
      </div>
      <SkeletonLines count={8} />
    </div>
  )
}

/** Sidebar loading skeleton */
export function SidebarSkeleton() {
  return (
    <div style={{ padding: 'var(--space-4)', display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
      <Skeleton width="60%" height="24px" />
      <Skeleton width="80%" />
      <Skeleton width="70%" />
      <Skeleton width="90%" />
      <Skeleton width="50%" />
      <Skeleton width="75%" />
    </div>
  )
}
