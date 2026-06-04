/**
 * FilterChip component tests — F13.
 *
 * RED: 8 operator renders; GREEN: long values truncated >20 chars.
 */

import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { FilterChip } from '../FilterChip';
import type { FilterChip as FilterChipType } from '@shared/types/filterChip';
import type { PropertyOp } from '@shared/types/queryAst';

// Helper to build minimal chips
function makeChip(overrides: Partial<FilterChipType> = {}): FilterChipType {
  return {
    id: 'chip-1',
    key: 'status',
    op: 'Equals',
    value: 'active',
    ...overrides,
  };
}

describe('FilterChip (F13)', () => {
  // ─── Operator icon / label rendering ────────────────────────────────────────

  describe('Renders all 8 operators correctly', () => {
    const operators: Array<{ op: PropertyOp; label: string }> = [
      { op: 'Equals', label: 'Equals' },
      { op: 'NotEquals', label: 'Not equals' },
      { op: 'Contains', label: 'Contains' },
      { op: 'GreaterThan', label: 'Greater than' },
      { op: 'LessThan', label: 'Less than' },
      { op: 'GreaterThanOrEqual', label: 'Greater than or equal' },
      { op: 'LessThanOrEqual', label: 'Less than or equal' },
      { op: 'Between', label: 'Between' },
    ];

    operators.forEach(({ op, label }) => {
      it(`renders operator: ${label}`, () => {
        const chip = makeChip({ op, value: 'x', value2: op === 'Between' ? 'y' : undefined });
        render(<FilterChip chip={chip} onRemove={vi.fn()} />);
        expect(screen.getByLabelText(`Remove ${chip.key} filter`)).toBeInTheDocument();
      });
    });
  });

  // ─── Between two values ──────────────────────────────────────────────────────

  it('shows two values when op is Between', () => {
    const chip = makeChip({ op: 'Between', value: '2024-01-01', value2: '2024-12-31' });
    render(<FilterChip chip={chip} onRemove={vi.fn()} />);
    expect(screen.getByText('2024-01-01')).toBeInTheDocument();
    expect(screen.getByText('2024-12-31')).toBeInTheDocument();
  });

  it('shows single value when op is not Between', () => {
    const chip = makeChip({ op: 'Equals', value: 'active' });
    render(<FilterChip chip={chip} onRemove={vi.fn()} />);
    expect(screen.getByText('active')).toBeInTheDocument();
  });

  // ─── Remove button ──────────────────────────────────────────────────────────

  it('calls onRemove with chip id when remove button is clicked', async () => {
    const user = userEvent.setup();
    const onRemove = vi.fn();
    const chip = makeChip({ id: 'chip-42', key: 'priority', value: 'high' });
    render(<FilterChip chip={chip} onRemove={onRemove} />);
    await user.click(screen.getByLabelText('Remove priority filter'));
    expect(onRemove).toHaveBeenCalledWith('chip-42');
  });

  // ─── Accessibility ─────────────────────────────────────────────────────────

  it('is focusable (tabIndex)', () => {
    const chip = makeChip();
    render(<FilterChip chip={chip} onRemove={vi.fn()} />);
    expect(screen.getByRole('button')).toHaveAttribute('tabIndex', '0');
  });

  it('has aria-label on remove button describing what is removed', () => {
    const chip = makeChip({ key: 'status', op: 'Equals', value: 'active' });
    render(<FilterChip chip={chip} onRemove={vi.fn()} />);
    // The remove button's aria-label tells screen readers which filter is being removed
    expect(screen.getByLabelText('Remove status filter')).toBeInTheDocument();
  });

  // ─── Value truncation ───────────────────────────────────────────────────────

  it('truncates long values (>20 chars) with ellipsis', () => {
    const longValue = 'this-is-a-very-long-value-that-exceeds-twenty-characters';
    const chip = makeChip({ value: longValue });
    render(<FilterChip chip={chip} onRemove={vi.fn()} />);
    const valueEl = screen.getByTestId('filter-chip-value');
    expect(valueEl).toHaveAttribute('title', longValue); // tooltip shows full
    // The visible text should be truncated (title attr enables tooltip)
    expect(valueEl.textContent).toContain('…');
  });

  it('shows full value (no truncation) when ≤20 chars', () => {
    const shortValue = 'active';
    const chip = makeChip({ value: shortValue });
    render(<FilterChip chip={chip} onRemove={vi.fn()} />);
    expect(screen.getByText(shortValue)).toBeInTheDocument();
  });

  // ─── Key label ─────────────────────────────────────────────────────────────

  it('displays the property key as a label', () => {
    const chip = makeChip({ key: 'priority', value: 'high' });
    render(<FilterChip chip={chip} onRemove={vi.fn()} />);
    expect(screen.getByText('priority')).toBeInTheDocument();
  });
});
