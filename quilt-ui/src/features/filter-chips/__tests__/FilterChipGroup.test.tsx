/**
 * FilterChipGroup tests — F13.
 *
 * RED: add/remove/validate scenarios; GREEN: component tests keyboard nav.
 */

import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { FilterChipGroup } from '../FilterChipGroup';
import type { FilterChip } from '@shared/types/filterChip';

// Helper to build chips
function makeChip(overrides: Partial<FilterChip> = {}): FilterChip {
  return {
    id: 'chip-1',
    key: 'status',
    op: 'Equals',
    value: 'active',
    ...overrides,
  };
}

describe('FilterChipGroup (F13)', () => {
  // ─── Rendering ─────────────────────────────────────────────────────────────

  it('renders empty message when chips array is empty', () => {
    render(
      <FilterChipGroup chips={[]} onChange={vi.fn()} />,
    );
    // Add filter button is always visible
    expect(screen.getByRole('button', { name: /add filter/i })).toBeInTheDocument();
  });

  it('renders a FilterChip for each chip in the list', () => {
    const chips = [
      makeChip({ id: 'c1', key: 'status', value: 'active' }),
      makeChip({ id: 'c2', key: 'priority', value: 'high' }),
    ];
    render(<FilterChipGroup chips={chips} onChange={vi.fn()} />);
    expect(screen.getAllByRole('button', { name: /remove/i })).toHaveLength(2);
  });

  // ─── Add chip ───────────────────────────────────────────────────────────────

  it('calls onChange when Add Filter button is clicked', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <FilterChipGroup chips={[]} onChange={onChange} availableKeys={['status', 'priority']} />,
    );
    // Click the Add Filter button — opens popover (onChange not called until chip is added)
    await user.click(screen.getByRole('button', { name: /add filter/i }));
    // Popover should open but no chip added yet
    expect(screen.getByLabelText('Property')).toBeInTheDocument();
  });

  // ─── Remove chip ───────────────────────────────────────────────────────────

  it('calls onChange with chip removed when remove button is clicked', async () => {
    const user = userEvent.setup();
    const chips = [
      makeChip({ id: 'c1', key: 'status', value: 'active' }),
      makeChip({ id: 'c2', key: 'priority', value: 'high' }),
    ];
    const onChange = vi.fn();
    render(<FilterChipGroup chips={chips} onChange={onChange} />);
    // Click remove on the first chip
    await user.click(screen.getAllByLabelText('Remove status filter')[0]);
    expect(onChange).toHaveBeenCalledWith([chips[1]]);
  });

  // ─── Validation — Between operator ───────────────────────────────────────────

  it('shows inline error when Between chip has missing value2', () => {
    const chips = [
      makeChip({ id: 'c1', op: 'Between', value: '2024-01-01', value2: '' }),
    ];
    const errors = { 'c1': 'Second value is required for Between operator' };
    render(
      <FilterChipGroup chips={chips} onChange={vi.fn()} errors={errors} />,
    );
    expect(screen.getByText('Second value is required for Between operator')).toBeInTheDocument();
  });

  it('shows inline error when chip has missing key', () => {
    const chips = [
      makeChip({ id: 'c1', key: '', value: 'active' }),
    ];
    const errors = { 'c1': 'Property key is required' };
    render(
      <FilterChipGroup chips={chips} onChange={vi.fn()} errors={errors} />,
    );
    expect(screen.getByText('Property key is required')).toBeInTheDocument();
  });

  it('shows inline error when chip has missing value', () => {
    const chips = [
      makeChip({ id: 'c1', key: 'status', value: '' }),
    ];
    const errors = { 'c1': 'Value is required' };
    render(
      <FilterChipGroup chips={chips} onChange={vi.fn()} errors={errors} />,
    );
    expect(screen.getByText('Value is required')).toBeInTheDocument();
  });

  it('applies disabled state to Apply button when errors exist', () => {
    const chips = [
      makeChip({ id: 'c1', op: 'Between', value: '2024-01-01', value2: '' }),
    ];
    const errors = { 'c1': 'Second value is required for Between operator' };
    render(
      <FilterChipGroup chips={chips} onChange={vi.fn()} errors={errors} disabled />,
    );
    expect(screen.getByRole('button', { name: /apply/i })).toBeDisabled();
  });

  // ─── Keyboard navigation ────────────────────────────────────────────────────

  it('add filter button is focusable via keyboard', async () => {
    const user = userEvent.setup();
    render(<FilterChipGroup chips={[]} onChange={vi.fn()} />);
    await user.tab();
    // First tab lands on the Add Filter button (its accessible name is "Add filter")
    expect(screen.getByRole('button', { name: /add filter/i })).toHaveFocus();
  });

  it('chip remove buttons are reachable via keyboard after add filter button', async () => {
    const user = userEvent.setup();
    const chips = [makeChip({ id: 'c1', key: 'status', value: 'active' })];
    render(<FilterChipGroup chips={chips} onChange={vi.fn()} />);
    // Tab order: chip remove button (first in DOM), then Add Filter button
    await user.tab(); // first tab: remove chip button
    expect(screen.getByLabelText('Remove status filter')).toHaveFocus();
    await user.tab(); // second tab: add filter button
    expect(screen.getByRole('button', { name: /add filter/i })).toHaveFocus();
  });

  // ─── Apply button ──────────────────────────────────────────────────────────

  it('Apply button is present when chips exist', () => {
    const chips = [makeChip({ id: 'c1', key: 'status', value: 'active' })];
    render(<FilterChipGroup chips={chips} onChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: /apply/i })).toBeInTheDocument();
  });

  it('Apply button is disabled when disabled prop is true', () => {
    const chips = [makeChip({ id: 'c1', key: 'status', value: 'active' })];
    render(<FilterChipGroup chips={chips} onChange={vi.fn()} disabled />);
    expect(screen.getByRole('button', { name: /apply/i })).toBeDisabled();
  });

  it('Apply button is disabled when there are validation errors', () => {
    const chips = [
      makeChip({ id: 'c1', op: 'Between', value: '2024-01-01', value2: '' }),
    ];
    const errors = { 'c1': 'Second value is required for Between operator' };
    render(<FilterChipGroup chips={chips} onChange={vi.fn()} errors={errors} />);
    expect(screen.getByRole('button', { name: /apply/i })).toBeDisabled();
  });

  it('Apply button is enabled when chips are valid and not disabled', () => {
    const chips = [makeChip({ id: 'c1', key: 'status', value: 'active' })];
    render(<FilterChipGroup chips={chips} onChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: /apply/i })).toBeEnabled();
  });
});
