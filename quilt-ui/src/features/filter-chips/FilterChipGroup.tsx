/**
 * FilterChipGroup — F13 filter chip group container.
 *
 * Renders:
 * - Ordered list of FilterChip components
 * - "Add Filter" button/popover
 * - Validation errors per chip
 * - Apply button (disabled when errors exist or disabled=true)
 */

import { useState } from 'react';
import { Plus } from 'lucide-react';
import { FilterChip } from './FilterChip';
import { validateFilterChip } from '@shared/types/filterChip';
import type { FilterChip as FilterChipType, FilterChipGroupProps } from '@shared/types/filterChip';
import type { PropertyOp } from '@shared/types/queryAst';


// ─── New chip defaults ────────────────────────────────────────────────────────

const DEFAULT_NEW_CHIP: Omit<FilterChipType, 'id'> = {
  key: '',
  op: 'Equals',
  value: '',
};

const OPERATOR_OPTIONS: PropertyOp[] = [
  'Equals',
  'NotEquals',
  'Contains',
  'GreaterThan',
  'LessThan',
  'GreaterThanOrEqual',
  'LessThanOrEqual',
  'Between',
];

// ─── Component ───────────────────────────────────────────────────────────────

export function FilterChipGroup({
  chips,
  onChange,
  addPopoverOpen,
  onAddPopoverOpen,
  onAddPopoverClose,
  availableKeys,
  disabled,
  errors = {},
  onApply,
}: FilterChipGroupProps) {
  const [popoverOpen, setPopoverOpen] = useState(addPopoverOpen ?? false);
  const [newChip, setNewChip] = useState<Omit<FilterChipType, 'id'>>(DEFAULT_NEW_CHIP);

  const openPopover = () => {
    setPopoverOpen(true);
    onAddPopoverOpen?.();
  };

  const closePopover = () => {
    setPopoverOpen(false);
    setNewChip(DEFAULT_NEW_CHIP);
    onAddPopoverClose?.();
  };

  const handleRemove = (id: string) => {
    onChange(chips.filter(c => c.id !== id));
  };

  const handleAddChip = () => {
    const validationError = validateFilterChip({ id: '', ...newChip } as FilterChipType);
    if (validationError) return; // Don't add invalid chips

    const added: FilterChipType = {
      ...newChip,
      id: crypto.randomUUID(),
    };
    onChange([...chips, added]);
    closePopover();
  };

  const hasErrors = Object.keys(errors).length > 0;
  const applyDisabled = disabled || hasErrors;

  return (
    <div className="flex flex-col gap-2">
      {/* Chip list */}
      <div className="flex flex-wrap items-center gap-2">
        {chips.map(chip => (
          <div key={chip.id} className="relative">
            <FilterChip chip={chip} onRemove={handleRemove} />
            {errors[chip.id] && (
              <p className="mt-1 text-xs text-destructive" role="alert">
                {errors[chip.id]}
              </p>
            )}
          </div>
        ))}

        {/* Add Filter button */}
        {!popoverOpen && (
          <button
            type="button"
            onClick={openPopover}
            disabled={disabled}
            className="inline-flex items-center gap-1 rounded-full border border-dashed border-muted-foreground/40 px-2.5 py-1 text-sm text-muted-foreground hover:border-muted-foreground hover:text-foreground disabled:opacity-50"
          >
            <Plus className="h-3 w-3" aria-hidden="true" />
            Add filter
          </button>
        )}
      </div>

      {/* Popover — inline form for new chip */}
      {popoverOpen && (
        <div className="flex flex-wrap items-end gap-2 rounded border bg-background p-3 shadow-sm">
          {/* Key selector */}
          <div className="flex flex-col gap-1">
            <label htmlFor="chip-key" className="text-xs font-medium text-muted-foreground">
              Property
            </label>
            {availableKeys && availableKeys.length > 0 ? (
              <select
                id="chip-key"
                value={newChip.key}
                onChange={e => setNewChip(c => ({ ...c, key: e.target.value }))}
                className="h-8 rounded border bg-background px-2 text-sm"
              >
                <option value="">Select…</option>
                {availableKeys.map(k => (
                  <option key={k} value={k}>{k}</option>
                ))}
              </select>
            ) : (
              <input
                id="chip-key"
                type="text"
                value={newChip.key}
                onChange={e => setNewChip(c => ({ ...c, key: e.target.value }))}
                placeholder="e.g. status"
                className="h-8 rounded border bg-background px-2 text-sm"
              />
            )}
          </div>

          {/* Operator selector */}
          <div className="flex flex-col gap-1">
            <label htmlFor="chip-op" className="text-xs font-medium text-muted-foreground">
              Operator
            </label>
            <select
              id="chip-op"
              value={newChip.op}
              onChange={e => setNewChip(c => ({ ...c, op: e.target.value as PropertyOp }))}
              className="h-8 rounded border bg-background px-2 text-sm"
            >
              {OPERATOR_OPTIONS.map(op => (
                <option key={op} value={op}>{op}</option>
              ))}
            </select>
          </div>

          {/* Value */}
          <div className="flex flex-col gap-1">
            <label htmlFor="chip-value" className="text-xs font-medium text-muted-foreground">
              Value
            </label>
            <input
              id="chip-value"
              type="text"
              value={newChip.value}
              onChange={e => setNewChip(c => ({ ...c, value: e.target.value }))}
              placeholder="Filter value"
              className="h-8 rounded border bg-background px-2 text-sm"
            />
          </div>

          {/* Value2 for Between */}
          {newChip.op === 'Between' && (
            <div className="flex flex-col gap-1">
              <label htmlFor="chip-value2" className="text-xs font-medium text-muted-foreground">
                And
              </label>
              <input
                id="chip-value2"
                type="text"
                value={newChip.value2 ?? ''}
                onChange={e => setNewChip(c => ({ ...c, value2: e.target.value }))}
                placeholder="Second value"
                className="h-8 rounded border bg-background px-2 text-sm"
              />
            </div>
          )}

          {/* Add / Cancel */}
          <div className="flex items-end gap-1">
            <button
              type="button"
              onClick={handleAddChip}
              className="h-8 rounded bg-primary px-3 text-sm text-primary-foreground hover:bg-primary/90"
            >
              Add
            </button>
            <button
              type="button"
              onClick={closePopover}
              className="h-8 rounded border px-3 text-sm hover:bg-muted"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Apply button */}
      {chips.length > 0 && (
        <div className="flex justify-end">
          <button
            type="button"
            disabled={applyDisabled}
            onClick={() => onApply?.(chips)}
            className="rounded bg-primary px-4 py-1.5 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            Apply
          </button>
        </div>
      )}
    </div>
  );
}
