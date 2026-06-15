//! DecorationSlots — renders visual decorations from a ProjectionView.
//!
//! Each decoration maps to a specific visual affordance (badge, checkbox,
//! date indicator, etc.). This component is intentionally dumb — it only
//! knows how to render slots based on `kind` and `weight`. The parent
//! (ProjectionRenderer) decides which decorations are visible and where.

import type { Decoration, DecorationKind } from './types';

// ─── Style helpers ─────────────────────────────────────────────────────────

const TASK_CHECKBOX_COLOR = 'var(--color-accent, #6366f1)';
const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  // Task markers
  todo: { bg: 'var(--color-info, #3b82f6)', text: '#fff' },
  doing: { bg: 'var(--color-accent, #6366f1)', text: '#fff' },
  done: { bg: 'var(--color-success, #22c55e)', text: '#fff' },
  now: { bg: 'var(--color-danger, #ef4444)', text: '#fff' },
  later: { bg: 'var(--color-warning, #f59e0b)', text: '#fff' },
  cancelled: { bg: 'var(--color-text-disabled, #9ca3af)', text: '#fff' },
  waiting: { bg: '#9333ea', text: '#fff' },
  // Generic statuses
  open: { bg: '#6b7280', text: '#fff' },
  closed: { bg: '#22c55e', text: '#fff' },
  'in-progress': { bg: '#3b82f6', text: '#fff' },
};

const WEIGHT_LABELS: Record<number, string> = {
  0: 'Low',
  128: 'Medium',
  200: 'High',
  255: 'Critical',
};

// ─── Slot renderers ────────────────────────────────────────────────────────

interface TaskCheckboxSlotProps {
  decoration: Decoration;
  onToggle?: (value: boolean) => void;
}

function TaskCheckboxSlot({ decoration, onToggle }: TaskCheckboxSlotProps) {
  const checked = decoration.value === true || decoration.value === 'true' ||
    String(decoration.value).toLowerCase() === 'done';

  return (
    <label
      data-testid="decoration-task-checkbox"
      data-target={decoration.target}
      data-weight={decoration.weight}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        cursor: onToggle ? 'pointer' : 'default',
        gap: '4px',
      }}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onToggle?.(e.target.checked)}
        aria-label={`Task: ${decoration.target}`}
        style={{
          width: '14px',
          height: '14px',
          accentColor: TASK_CHECKBOX_COLOR,
          cursor: onToggle ? 'pointer' : 'default',
        }}
      />
    </label>
  );
}

interface StatusBadgeSlotProps {
  decoration: Decoration;
}

function StatusBadgeSlot({ decoration }: StatusBadgeSlotProps) {
  const statusKey = String(decoration.value).toLowerCase();
  const colors = STATUS_COLORS[statusKey] ?? { bg: '#6b7280', text: '#fff' };

  return (
    <span
      data-testid="decoration-status-badge"
      data-target={decoration.target}
      data-weight={decoration.weight}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        fontSize: '11px',
        fontWeight: 600,
        padding: '2px 8px',
        borderRadius: 'var(--radius-pill, 9999px)',
        background: colors.bg,
        color: colors.text,
        letterSpacing: '0.01em',
        lineHeight: 1.4,
      }}
    >
      {String(decoration.value).toUpperCase()}
    </span>
  );
}

interface GenericBadgeSlotProps {
  decoration: Decoration;
}

function GenericBadgeSlot({ decoration }: GenericBadgeSlotProps) {
  return (
    <span
      data-testid="decoration-generic-badge"
      data-target={decoration.target}
      data-weight={decoration.weight}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        fontSize: '11px',
        fontWeight: 500,
        padding: '2px 8px',
        borderRadius: 'var(--radius-pill, 9999px)',
        background: 'var(--color-surface-subtle, #f3f4f6)',
        color: 'var(--color-text-secondary, #374151)',
        letterSpacing: '0.01em',
        lineHeight: 1.4,
      }}
    >
      {String(decoration.value)}
    </span>
  );
}

interface DateIndicatorSlotProps {
  decoration: Decoration;
}

function DateIndicatorSlot({ decoration }: DateIndicatorSlotProps) {
  const dateStr = String(decoration.value);
  // Format ISO date for display
  let display = dateStr;
  try {
    const d = new Date(dateStr);
    if (!isNaN(d.getTime())) {
      display = d.toLocaleDateString(undefined, {
        month: 'short',
        day: 'numeric',
      });
    }
  } catch {
    // Use raw value
  }

  return (
    <span
      data-testid="decoration-date-indicator"
      data-target={decoration.target}
      data-weight={decoration.weight}
      title={dateStr}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '4px',
        fontSize: '11px',
        color: 'var(--color-text-muted, #6b7280)',
      }}
    >
      <span aria-hidden="true">📅</span>
      <span>{display}</span>
    </span>
  );
}

interface LinkAffordanceSlotProps {
  decoration: Decoration;
  links: Array<{ url: string; label: string }>;
}

function LinkAffordanceSlot({ decoration, links }: LinkAffordanceSlotProps) {
  const link = links.find((l) => l.url === decoration.value || l.label === decoration.value);

  if (!link) return null;

  return (
    <a
      href={link.url}
      target="_blank"
      rel="noopener noreferrer"
      data-testid="decoration-link-affordance"
      data-target={decoration.target}
      data-weight={decoration.weight}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '2px',
        fontSize: '11px',
        color: 'var(--color-accent, #6366f1)',
        textDecoration: 'none',
      }}
    >
      <span aria-hidden="true">🔗</span>
      <span>{link.label || new URL(link.url).hostname}</span>
    </a>
  );
}

// ─── Main component ─────────────────────────────────────────────────────────

export interface DecorationSlotsProps {
  /** Decorations from the projection view. */
  decorations: Decoration[];
  /** Links from the projection view (for link affordances). */
  links?: Array<{ url: string; label: string }>;
  /** Called when a task checkbox is toggled. */
  onTaskToggle?: (target: string, checked: boolean) => void;
  /** Optional filter to only show specific decoration kinds. */
  filterKinds?: DecorationKind[];
  /** Optional filter to show only decorations targeting specific property keys. */
  filterTargets?: string[];
  /** CSS class to apply to the container. */
  className?: string;
}

/**
 * Render visual decorations from a ProjectionView.
 *
 * Each decoration maps to a specific slot renderer. The component is
 * intentionally stateless — it only renders based on the decoration data.
 */
export function DecorationSlots({
  decorations,
  links = [],
  onTaskToggle,
  filterKinds,
  filterTargets,
  className,
}: DecorationSlotsProps) {
  const filtered = decorations.filter((d) => {
    if (filterKinds && !filterKinds.includes(d.kind)) return false;
    if (filterTargets && !filterTargets.includes(d.target)) return false;
    return true;
  });

  // Sort by weight descending so higher-priority decorations render first
  const sorted = [...filtered].sort((a, b) => b.weight - a.weight);

  if (sorted.length === 0) return null;

  return (
    <span
      data-testid="decoration-slots"
      className={className}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '6px',
        flexWrap: 'wrap',
      }}
    >
      {sorted.map((decoration, i) => {
        switch (decoration.kind) {
          case 'task-checkbox':
            return (
              <TaskCheckboxSlot
                key={`${decoration.target}-${i}`}
                decoration={decoration}
                onToggle={
                  onTaskToggle
                    ? (checked) => onTaskToggle(decoration.target, checked)
                    : undefined
                }
              />
            );

          case 'status-badge':
            return (
              <StatusBadgeSlot
                key={`${decoration.target}-${i}`}
                decoration={decoration}
              />
            );

          case 'generic-badge':
            return (
              <GenericBadgeSlot
                key={`${decoration.target}-${i}`}
                decoration={decoration}
              />
            );

          case 'date-indicator':
            return (
              <DateIndicatorSlot
                key={`${decoration.target}-${i}`}
                decoration={decoration}
              />
            );

          case 'link-affordance':
            return (
              <LinkAffordanceSlot
                key={`${decoration.target}-${i}`}
                decoration={decoration}
                links={links}
              />
            );

          // Unsupported decoration kinds are silently skipped
          default:
            return null;
        }
      })}
    </span>
  );
}
