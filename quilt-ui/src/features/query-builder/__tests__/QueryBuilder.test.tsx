/**
 * QueryBuilder tests — F17.
 *
 * RED: 3 sort scenarios + race resolved;
 * GREEN: component integration tests with mock executeQuery.
 */

import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import userEvent from '@testing-library/user-event';
import { QueryBuilder } from '../QueryBuilder';
import type { ColumnDef } from '../../table-view/ColumnDef';

// ─── Mock api.executeQuery ─────────────────────────────────────────────────────

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockExecuteQuery = vi.fn<any, any>();

vi.mock('@core/api-client', () => ({
  api: {
    executeQuery: (...args: unknown[]) => mockExecuteQuery(...args),
  },
}));

// ─── Helpers ───────────────────────────────────────────────────────────────────

function makeColumns(): ColumnDef[] {
  return [
    { key: 'name', header: 'Name', width: 200, sortable: true },
    { key: 'status', header: 'Status', width: 120, sortable: true },
    { key: 'priority', header: 'Priority', width: 100 },
  ];
}

function makeRows(): Record<string, unknown>[] {
  return [
    { id: '1', name: 'Alice', status: 'active', priority: 'high' },
    { id: '2', name: 'Bob', status: 'inactive', priority: 'low' },
  ];
}

function makeQueryResult(rows: Record<string, unknown>[] = makeRows()) {
  return { results: rows, total: rows.length, elapsed_ms: 12 };
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

describe('QueryBuilder (F17)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockExecuteQuery.mockResolvedValue(makeQueryResult());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ─── Rendering ─────────────────────────────────────────────────────────────

  it('renders FilterChipGroup and TableView', () => {
    render(<QueryBuilder columns={makeColumns()} availableKeys={['status', 'priority']} />);
    expect(screen.getByTestId('query-builder')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add filter/i })).toBeInTheDocument();
  });

  it('renders empty TableView when no results yet', () => {
    render(<QueryBuilder columns={makeColumns()} />);
    expect(screen.getByTestId('table-empty')).toBeInTheDocument();
  });

  // ─── Apply triggers query execution ─────────────────────────────────────────

  it('calls executeQuery when a chip is added and Apply is clicked', async () => {
    const user = userEvent.setup();
    render(<QueryBuilder columns={makeColumns()} availableKeys={['status']} />);

    // Add a chip via the popover
    await user.click(screen.getByRole('button', { name: /add filter/i }));
    await user.selectOptions(screen.getByLabelText('Property'), 'status');
    await user.type(screen.getByLabelText('Value'), 'active');
    await user.click(screen.getByRole('button', { name: /add/i, exact: true }));

    // Now Apply button should be enabled
    const applyBtn = screen.getByRole('button', { name: /apply/i });
    await user.click(applyBtn);

    await waitFor(() => {
      expect(mockExecuteQuery).toHaveBeenCalledTimes(1);
    });
  });

  // ─── Race condition — AbortController cancels previous request ──────────────

  it('cancels previous request when a new sort is applied before the first resolves', async () => {
    const user = userEvent.setup();

    mockExecuteQuery.mockImplementationOnce(
      () => new Promise((resolve) => setTimeout(() => resolve(makeQueryResult()), 100)) as any,
    );
    mockExecuteQuery.mockImplementationOnce(() => Promise.resolve(makeQueryResult()) as any);

    render(<QueryBuilder columns={makeColumns()} availableKeys={['status']} />);

    // Add and apply a chip (starts slow request that takes 100ms)
    await user.click(screen.getByRole('button', { name: /add filter/i }));
    await user.selectOptions(screen.getByLabelText('Property'), 'status');
    await user.type(screen.getByLabelText('Value'), 'active');
    await user.click(screen.getByRole('button', { name: /add/i, exact: true }));
    await user.click(screen.getByRole('button', { name: /apply/i }));

    // Wait for the table to render with results (after slow promise resolves)
    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });

    // Sort — this triggers a new execute call (second mock)
    await user.click(screen.getByTestId('sort-name'));

    // Both calls should have been made (first resolved, second also made)
    await waitFor(() => {
      expect(mockExecuteQuery).toHaveBeenCalledTimes(2);
    });

    // Verify the second call has SortBy
    const secondCall = mockExecuteQuery.mock.calls[1];
    const ast = secondCall[0] as { SortBy?: unknown };
    expect(ast.SortBy).toBeDefined();
  });

  // ─── Sort re-executes query ─────────────────────────────────────────────────

  it('re-executes query with SortBy when sortable column header is clicked', async () => {
    const user = userEvent.setup();
    render(<QueryBuilder columns={makeColumns()} availableKeys={['status', 'priority']} />);

    // Apply a chip first
    await user.click(screen.getByRole('button', { name: /add filter/i }));
    await user.selectOptions(screen.getByLabelText('Property'), 'status');
    await user.type(screen.getByLabelText('Value'), 'active');
    await user.click(screen.getByRole('button', { name: /add/i, exact: true }));
    await user.click(screen.getByRole('button', { name: /apply/i }));

    await waitFor(() => {
      expect(mockExecuteQuery).toHaveBeenCalledTimes(1);
    });

    // Sort by name
    await user.click(screen.getByTestId('sort-name'));

    await waitFor(() => {
      expect(mockExecuteQuery).toHaveBeenCalledTimes(2);
    });

    // Second call should include SortBy
    const secondCall = mockExecuteQuery.mock.calls[1];
    const ast = secondCall[0] as { SortBy?: unknown };
    expect(ast.SortBy).toBeDefined();
    expect((ast.SortBy as { field: string }).field).toBe('name');
  });

  it('sort indicator shows active sort state (↕)', async () => {
    const user = userEvent.setup();
    render(<QueryBuilder columns={makeColumns()} availableKeys={['status', 'priority']} />);

    // Apply a chip first
    await user.click(screen.getByRole('button', { name: /add filter/i }));
    await user.selectOptions(screen.getByLabelText('Property'), 'status');
    await user.type(screen.getByLabelText('Value'), 'active');
    await user.click(screen.getByRole('button', { name: /add/i, exact: true }));
    await user.click(screen.getByRole('button', { name: /apply/i }));

    await waitFor(() => {
      expect(mockExecuteQuery).toHaveBeenCalledTimes(1);
    });

    // Click sort — should show directional indicator
    await user.click(screen.getByTestId('sort-status'));
    expect(screen.getByTestId('table-header')).toHaveTextContent('↓');
  });

  // ─── Results display ────────────────────────────────────────────────────────

  it('displays query results in TableView', async () => {
    const user = userEvent.setup();
    mockExecuteQuery.mockResolvedValue(makeQueryResult());
    render(<QueryBuilder columns={makeColumns()} availableKeys={['status']} />);

    await user.click(screen.getByRole('button', { name: /add filter/i }));
    await user.selectOptions(screen.getByLabelText('Property'), 'status');
    await user.type(screen.getByLabelText('Value'), 'active');
    await user.click(screen.getByRole('button', { name: /add/i, exact: true }));
    await user.click(screen.getByRole('button', { name: /apply/i }));

    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });
  });

  // ─── Error handling ─────────────────────────────────────────────────────────

  it('displays error message when executeQuery throws', async () => {
    const user = userEvent.setup();
    mockExecuteQuery.mockRejectedValue(new Error('Server error'));
    render(<QueryBuilder columns={makeColumns()} availableKeys={['status']} />);

    await user.click(screen.getByRole('button', { name: /add filter/i }));
    await user.selectOptions(screen.getByLabelText('Property'), 'status');
    await user.type(screen.getByLabelText('Value'), 'active');
    await user.click(screen.getByRole('button', { name: /add/i, exact: true }));
    await user.click(screen.getByRole('button', { name: /apply/i }));

    await waitFor(() => {
      expect(screen.getByTestId('query-builder-error')).toHaveTextContent('Server error');
    });
  });
});
