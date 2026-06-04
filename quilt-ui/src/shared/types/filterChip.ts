/**
 * FilterChip types — F13 filter chips component.
 *
 * Mirrors `quilt_query::property_op::PropertyOp` (8 variants).
 * These chips represent property-based query filters in the UI.
 */

import type { PropertyOp } from './queryAst';

// ─── FilterChip ─────────────────────────────────────────────────────────────────

/**
 * One filter chip — represents a single property filter condition.
 * ID is a client-side UUID v4 (stable across renders).
 */
export interface FilterChip {
  /** Client-side generated UUID v4 — stable across renders. */
  id: string;
  /** Property key to filter on (e.g., "status", "priority"). */
  key: string;
  /** Comparison operator. */
  op: PropertyOp;
  /** Primary value to compare against. */
  value: string;
  /**
   * Secondary value — required for `Between` operator.
   * Undefined for all other operators.
   */
  value2?: string;
}

// ─── FilterChipGroupProps ──────────────────────────────────────────────────────

/** Props for the FilterChipGroup container component. */
export interface FilterChipGroupProps {
  /**
   * Ordered list of active filter chips.
   * AND-combined when multiple chips are present.
   */
  chips: FilterChip[];
  /**
   * Callback fired when the chip list changes (add, remove, or update).
   * Called with the new ordered list of chips.
   */
  onChange: (chips: FilterChip[]) => void;
  /** Whether the "Add filter" popover is open. */
  addPopoverOpen?: boolean;
  /** Callback to open the add filter popover. */
  onAddPopoverOpen?: () => void;
  /** Callback to close the add filter popover. */
  onAddPopoverClose?: () => void;
  /** Available property keys for the dropdown. */
  availableKeys?: string[];
  /** Whether to disable the Apply button (e.g., during query execution). */
  disabled?: boolean;
  /** Error messages per chip (chip.id → error string). */
  errors?: Record<string, string>;
}

// ─── Validation ────────────────────────────────────────────────────────────────

/**
 * Validate a single FilterChip.
 * Returns an error message if invalid, or undefined if valid.
 */
export function validateFilterChip(chip: FilterChip): string | undefined {
  if (!chip.key.trim()) {
    return 'Property key is required';
  }
  if (!chip.value.trim()) {
    return 'Value is required';
  }
  if (chip.op === 'Between') {
    if (!chip.value2?.trim()) {
      return 'Second value is required for Between operator';
    }
  }
  return undefined;
}

/**
 * Validate the entire chip list.
 * Returns a map of chip ID → error message for invalid chips.
 */
export function validateChipList(chips: FilterChip[]): Record<string, string> {
  const errors: Record<string, string> = {};
  for (const chip of chips) {
    const error = validateFilterChip(chip);
    if (error) {
      errors[chip.id] = error;
    }
  }
  return errors;
}

/**
 * Check if the chip list is valid enough to execute a query.
 * True when there are no validation errors.
 */
export function isChipListValid(chips: FilterChip[]): boolean {
  return Object.keys(validateChipList(chips)).length === 0;
}
