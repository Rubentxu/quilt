/**
 * TableView tests — F17.
 *
 * RED: 1000 rows → ≤50 <tr> in DOM via getAllByRole('row');
 * GREEN: render perf — virtual window only.
 */

import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { TableView } from '../TableView';
import type { ColumnDef } from '../ColumnDef';

// Helper to build column defs
function makeColumns(): ColumnDef[] {
  return [
    { key: 'name', header: 'Name', width: 150 },
    { key: 'status', header: 'Status', width: 120, sortable: true },
    { key: 'priority', header: 'Priority', width: 100 },
  ];
}

// Generate N rows of data
function makeRows(n: number): Record<string, unknown>[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `row-${i}`,
    name: `Item ${i}`,
    status: i % 2 === 0 ? 'active' : 'inactive',
    priority: ['high', 'medium', 'low'][i % 3] as string,
  }));
}

describe('TableView (F17)', () => {
  // ─── Virtual rendering — only visible window in DOM ─────────────────────────

  it('renders 1000 rows with ≤50 <tr> in DOM (virtual window)', () => {
    const columns = makeColumns();
    const rows = makeRows(1000);
    render(<TableView columns={columns} rows={rows} />);
    // react-virtuoso renders a limited window; count all table rows in DOM
    const rows_all = document.querySelectorAll('[role="row"]');
    expect(rows_all.length).toBeLessThanOrEqual(50);
  });

  // ─── Empty state ───────────────────────────────────────────────────────────

  it('shows "No results" when rows array is empty', () => {
    const columns = makeColumns();
    render(<TableView columns={columns} rows={[]} />);
    expect(screen.getByText('No results')).toBeInTheDocument();
  });

  // ─── Column headers ────────────────────────────────────────────────────────

  it('renders all column headers when rows exist', () => {
    const columns = makeColumns();
    const rows = makeRows(5);
    render(<TableView columns={columns} rows={rows} />);
    const header = screen.getByTestId('table-header');
    expect(header).toHaveTextContent('Name');
    expect(header).toHaveTextContent('Status');
    expect(header).toHaveTextContent('Priority');
  });

  // ─── Cell values ───────────────────────────────────────────────────────────

  it('renders cell values in visible rows', () => {
    const columns = makeColumns();
    const rows = [
      { id: '1', name: 'Alice', status: 'active', priority: 'high' },
    ];
    render(<TableView columns={columns} rows={rows} />);
    // Virtuoso renders rows in the viewport — use visible text instead of DOM queries
    expect(screen.getByText('Alice')).toBeInTheDocument();
  });

  // ─── Sort ─────────────────────────────────────────────────────────────────

  it('calls onSort with Desc on first click (toggle from initial)', async () => {
    const { default: userEvent } = await import('@testing-library/user-event');
    const columns = makeColumns(); // status is sortable
    const rows = makeRows(10);
    const onSort = vi.fn();
    render(<TableView columns={columns} rows={rows} onSort={onSort} />);
    // Initial sort: undefined (no sort), first click should be Desc
    await userEvent.click(screen.getByTestId('sort-status'));
    expect(onSort).toHaveBeenCalledWith('status', 'Desc');
  });

  it('calls onSort with Asc on second click', async () => {
    const { default: userEvent } = await import('@testing-library/user-event');
    const columns = makeColumns(); // status is sortable
    const rows = makeRows(10);
    const onSort = vi.fn();
    render(<TableView columns={columns} rows={rows} onSort={onSort} />);
    await userEvent.click(screen.getByTestId('sort-status')); // Desc
    await userEvent.click(screen.getByTestId('sort-status')); // Asc
    expect(onSort).toHaveBeenLastCalledWith('status', 'Asc');
  });

  it('renders sort indicator (↕) in sortable column header', () => {
    const columns = makeColumns(); // status is sortable
    const rows = makeRows(5);
    render(<TableView columns={columns} rows={rows} />);
    // The sort indicator ↕ should be visible for the sortable column
    expect(screen.getByTestId('table-header')).toHaveTextContent('↕');
  });

  // ─── Custom render function ───────────────────────────────────────────────

  it('uses custom render function when provided', () => {
    const columns: ColumnDef[] = [
      {
        key: 'status',
        header: 'Status',
        width: 120,
        render: (value: unknown) => {
          const v = String(value);
          return <span data-testid={`status-${v}`}>{v.toUpperCase()}</span>;
        },
      },
    ];
    const rows = [{ id: '1', status: 'active' }];
    render(<TableView columns={columns} rows={rows} />);
    // Verify header still shows column name
    expect(screen.getByTestId('table-header')).toHaveTextContent('Status');
    // The custom renderer's output should appear in the table (ACTIVE)
    expect(screen.getByText('ACTIVE')).toBeInTheDocument();
  });
});
