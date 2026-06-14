import type { TaskMarker } from '@shared/types/api'

const MARKER_CHECKED_STATE: Record<TaskMarker, boolean | 'indeterminate'> = {
  todo: false,
  doing: 'indeterminate',
  done: true,
  now: 'indeterminate',
  later: false,
  cancelled: true,
}

const MARKER_COLORS: Record<TaskMarker, string> = {
  todo: 'var(--color-info)',
  doing: 'var(--color-accent)',
  done: 'var(--color-success)',
  now: 'var(--color-danger)',
  later: 'var(--color-warning)',
  cancelled: 'var(--color-text-disabled)',
}

interface CheckboxProps {
  marker: TaskMarker
  onChange: () => void
}

export function BlockCheckbox({ marker, onChange }: CheckboxProps) {
  const checked = MARKER_CHECKED_STATE[marker]
  const color = MARKER_COLORS[marker]

  return (
    <button
      role="checkbox"
      aria-checked={checked === 'indeterminate' ? 'mixed' : checked}
      aria-label={`Task status: ${marker}`}
      onClick={e => {
        e.stopPropagation()
        onChange()
      }}
      style={{
        width: '16px',
        height: '16px',
        borderRadius: 'var(--radius-sm)',
        border: `2px solid ${color}`,
        background: checked === true ? color : 'transparent',
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 0,
        flexShrink: 0,
        transition: 'all 0.15s ease',
      }}
    >
      {checked === true && (
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
          <path
            d="M2 5L4 7L8 3"
            stroke="#fff"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      )}
      {checked === 'indeterminate' && (
        <div
          style={{
            width: '8px',
            height: '2px',
            background: color,
            borderRadius: '1px',
          }}
        />
      )}
    </button>
  )
}
