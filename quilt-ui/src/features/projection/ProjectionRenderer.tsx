//! ProjectionRenderer — renders a block using its resolved ProjectionView.
//!
//! This component is the "V1 contract output" for block rendering. It takes
//! a `ProjectionView` (from `GET /api/v1/blocks/:id/projection`) and produces
//! the visual representation: text content + decorations + conflict indicators.
//!
//! The component is intentionally side-effect free — it receives all data
//! via props and emits events (task toggle, link click) upward.
//!
//! # V1 Contract outputs
//!
//! 1. **text** — raw text content rendered inline
//! 2. **decorations** — visual annotations (badges, checkboxes, date indicators)
//! 3. **conflicts** — when resolution was ambiguous, shows conflict indicator
//! 4. **edit mode** — when `isEditing`, shows a contentEditable div
//! 5. **links** — rendered as affordances within the text or as separate slots
//! 6. **a11y** — proper ARIA roles and labels throughout

import { useCallback } from 'react';
import type {
  ProjectionView as ProjectionViewType,
  Decoration,
  DecorationKind,
} from './types';
import { DecorationSlots } from './DecorationSlots';

// ─── Conflict indicator ────────────────────────────────────────────────────

interface ConflictIndicatorProps {
  reason: string;
  candidates: string[];
}

function ConflictIndicator({ reason, candidates }: ConflictIndicatorProps) {
  return (
    <span
      data-testid="projection-conflict"
      role="alert"
      aria-label="Projection conflict"
      title={`Conflict: ${reason}. Candidates: ${candidates.join(', ')}`}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '4px',
        fontSize: '11px',
        color: 'var(--color-warning, #f59e0b)',
        background: 'rgba(245, 158, 11, 0.1)',
        padding: '2px 6px',
        borderRadius: 'var(--radius-sm, 4px)',
      }}
    >
      <span aria-hidden="true">⚠️</span>
      <span>Conflict</span>
    </span>
  );
}

// ─── Render mode ───────────────────────────────────────────────────────────

export type ProjectionRenderMode = 'read' | 'edit';

/**
 * Props for the ProjectionRenderer component.
 */
export interface ProjectionRendererProps {
  /**
   * The resolved projection view to render.
   * When null, renders a loading skeleton.
   */
  projection: ProjectionViewType | null;
  /** Whether the block is in edit mode. */
  isEditing?: boolean;
  /** Current text content (for edit mode). */
  content?: string;
  /** Called when the user starts editing (clicks the content). */
  onStartEdit?: () => void;
  /** Called when the user finishes editing (blurs the content). */
  onEndEdit?: (text: string) => void;
  /** Called when a task checkbox is toggled. */
  onTaskToggle?: (target: string, checked: boolean) => void;
  /** Additional CSS class for the root element. */
  className?: string;
  /** CSS style for the root element. */
  style?: React.CSSProperties;
  /** Test ID for the root element. */
  testId?: string;
}

// ─── Main renderer ─────────────────────────────────────────────────────────

/**
 * Render a block using its resolved projection view.
 *
 * Produces the 6 V1 contract outputs:
 * 1. text — the block's text content
 * 2. decorations — visual annotations
 * 3. conflicts — conflict indicators
 * 4. edit mode — contentEditable when isEditing=true
 * 5. links — rendered as affordances
 * 6. a11y — proper ARIA semantics
 */
export function ProjectionRenderer({
  projection,
  isEditing = false,
  content,
  onStartEdit,
  onEndEdit,
  onTaskToggle,
  className,
  style,
  testId = 'projection-renderer',
}: ProjectionRendererProps) {
  // Loading state
  if (!projection) {
    return (
      <span
        data-testid={testId}
        className={className}
        style={style}
        aria-busy="true"
        aria-label="Loading projection"
      >
        <span
          data-testid="projection-skeleton"
          style={{
            display: 'inline-block',
            width: '120px',
            height: '1em',
            background: 'var(--color-surface-subtle, #f3f4f6)',
            borderRadius: '4px',
            opacity: 0.6,
          }}
        />
      </span>
    );
  }

  // Edit mode
  if (isEditing) {
    return (
      <span
        data-testid={testId}
        data-mode="edit"
        className={className}
        style={style}
      >
        <span
          contentEditable
          suppressContentEditableWarning
          role="textbox"
          aria-multiline="false"
          aria-label="Block content"
          data-testid="projection-content-editable"
          onBlur={(e) => {
            const text = e.currentTarget.textContent ?? '';
            onEndEdit?.(text);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              e.currentTarget.blur();
            }
          }}
          style={{
            outline: 'none',
            minHeight: '1.5em',
            wordBreak: 'break-word',
            whiteSpace: 'pre-wrap',
            color: 'var(--color-text-primary, #111827)',
          }}
        >
          {content ?? projection.text}
        </span>
      </span>
    );
  }

  // Read mode — render text with decorations
  return (
    <span
      data-testid={testId}
      data-mode="read"
      className={className}
      style={{
        display: 'inline',
        ...style,
      }}
    >
      {/* Text content */}
      <span data-testid="projection-text" style={{ wordBreak: 'break-word' }}>
        {projection.text || '\u00A0'}{' '}
      </span>

      {/* Decorations */}
      {projection.decorations.length > 0 && (
        <DecorationSlots
          decorations={projection.decorations}
          links={projection.links.map((l) => ({ url: l.url, label: l.label }))}
          onTaskToggle={onTaskToggle}
        />
      )}

      {/* Conflict indicator */}
      {projection.conflicts.map((conflict, i) => (
        <ConflictIndicator
          key={`conflict-${i}`}
          reason={conflict.reason}
          candidates={conflict.candidates}
        />
      ))}
    </span>
  );
}

// ─── Preset application indicator ───────────────────────────────────────────

export interface PresetAppliedProps {
  presetId: string;
  label?: string;
}

/**
 * Indicator that a preset was recently applied.
 * Shown briefly after applying a preset via the slash command.
 */
export function PresetAppliedIndicator({ presetId, label }: PresetAppliedProps) {
  return (
    <span
      data-testid="preset-applied-indicator"
      data-preset-id={presetId}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '4px',
        fontSize: '11px',
        color: 'var(--color-success, #22c55e)',
        background: 'rgba(34, 197, 94, 0.1)',
        padding: '2px 6px',
        borderRadius: 'var(--radius-sm, 4px)',
        marginLeft: '4px',
      }}
    >
      <span aria-hidden="true">✨</span>
      <span>{label ?? presetId}</span>
    </span>
  );
}
