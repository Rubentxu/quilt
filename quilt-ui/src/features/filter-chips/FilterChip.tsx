/**
 * FilterChip — F13 filter chip pill component.
 *
 * Renders a single filter condition as a focusable pill with:
 * - Property key label
 * - Operator icon
 * - Value (truncated >20 chars)
 * - Remove button
 */

import { useState } from 'react';
import { X } from 'lucide-react';
import type { FilterChip as FilterChipType } from '@shared/types/filterChip';
import type { PropertyOp } from '@shared/types/queryAst';

// ─── Operator metadata ─────────────────────────────────────────────────────────

const OPERATOR_LABELS: Record<PropertyOp, string> = {
  Equals: 'Equals',
  NotEquals: 'Not equals',
  Contains: 'Contains',
  GreaterThan: 'Greater than',
  LessThan: 'Less than',
  GreaterThanOrEqual: 'Greater than or equal',
  LessThanOrEqual: 'Less than or equal',
  Between: 'Between',
};

// ─── Truncation ───────────────────────────────────────────────────────────────

const TRUNCATE_AT = 20;

function truncate(value: string, maxLen = TRUNCATE_AT): string {
  if (value.length <= maxLen) return value;
  return value.slice(0, maxLen - 1) + '…';
}

// ─── Component ───────────────────────────────────────────────────────────────

export interface FilterChipProps {
  chip: FilterChipType;
  onRemove: (id: string) => void;
}

/**
 * A single filter chip pill — displays key/op/value and a remove button.
 *
 * Accessibility:
 * - Renders as a `<button>` so it is focusable and activated via keyboard.
 * - `aria-label` describes the full filter text for screen readers.
 */
export function FilterChip({ chip, onRemove }: FilterChipProps) {
  const { id, key, op, value, value2 } = chip;

  const handleRemove = () => onRemove(id);

  const ariaLabel = `${key} ${OPERATOR_LABELS[op]} ${
    op === 'Between' && value2 ? `${value} and ${value2}` : value
  } — remove filter`;

  return (
    <div
      role="group"
      aria-label={`Filter: ${key} ${OPERATOR_LABELS[op]}`}
      className="inline-flex items-center gap-1 rounded-full bg-muted px-2.5 py-1 text-sm"
    >
      {/* Property key */}
      <span className="font-medium text-foreground">{key}</span>

      {/* Operator */}
      <span className="text-muted-foreground">{OPERATOR_LABELS[op]}</span>

      {/* Value(s) */}
      <span
        data-testid="filter-chip-value"
        className="max-w-[200px] truncate text-foreground"
        title={value} // tooltip for truncated values
      >
        {truncate(value)}
      </span>

      {/* Between second value */}
      {op === 'Between' && value2 && (
        <>
          <span className="text-muted-foreground">and</span>
          <span className="max-w-[200px] truncate text-foreground" title={value2}>
            {truncate(value2)}
          </span>
        </>
      )}

      {/* Remove button */}
      <button
        type="button"
        aria-label={`Remove ${key} filter`}
        onClick={handleRemove}
        className="ml-1 rounded-full p-0.5 text-muted-foreground hover:bg-muted-foreground/20 focus:outline-none focus:ring-2 focus:ring-ring"
        tabIndex={0}
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}
