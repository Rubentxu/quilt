/**
 * TableView — F17 virtualized table component.
 *
 * Uses react-virtuoso TableVirtuoso to render only the visible window,
 * ensuring that even with 1000+ rows, the DOM stays small (≤50 <tr>).
 *
 * Structure:
 * - Header: rendered as a real <thead> with sticky positioning via fixedHeaderContent
 * - Body: virtuoso-rendered <tbody> with visible window only
 */

import { useState } from 'react';
import { TableVirtuoso } from 'react-virtuoso';
import type { ColumnDef } from './ColumnDef';
import type { SortDirection } from '@shared/types/queryAst';

export interface TableViewProps {
  /** Column definitions. */
  columns: ColumnDef[];
  /** Row data — each row is a record keyed by column `key`. */
  rows: Record<string, unknown>[];
  /** Called when a sortable column header is clicked. */
  onSort?: (key: string, direction: SortDirection) => void;
  /** Initial sort key. */
  initialSortKey?: string;
  initialSortDir?: SortDirection;
}

// ─── Default cell renderer ─────────────────────────────────────────────────────

function DefaultCell({ value }: { value: unknown }): React.ReactNode {
  if (value === null || value === undefined) return <span className="text-muted-foreground">—</span>;
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  return String(value);
}

// ─── Sort toggle logic ─────────────────────────────────────────────────────────
// Toggle: initial → Desc → Asc → Desc
// - Not sorted: first click = Desc
// - Sorted Asc: next click = Desc
// - Sorted Desc: next click = Asc

function nextSortDir(current: SortDirection | undefined, isSorted: boolean): SortDirection {
  if (!isSorted) return 'Desc';
  return current === 'Desc' ? 'Asc' : 'Desc';
}

// ─── TableView ───────────────────────────────────────────────────────────────

export function TableView({
  columns,
  rows,
  onSort,
  initialSortKey,
  initialSortDir = 'Asc',
}: TableViewProps) {
  const [sortKey, setSortKey] = useState<string | undefined>(initialSortKey);
  const [sortDir, setSortDir] = useState<SortDirection | undefined>(initialSortDir);

  const handleSort = (key: string, dir: SortDirection) => {
    setSortKey(key);
    setSortDir(dir);
    onSort?.(key, dir);
  };

  if (rows.length === 0) {
    return (
      <div
        data-testid="table-empty"
        className="flex h-64 items-center justify-center text-muted-foreground"
      >
        No results
      </div>
    );
  }

  return (
    <div
      data-testid="table-container"
      className="relative overflow-auto rounded border"
      style={{ height: 400 }}
    >
      <TableVirtuoso
        data={rows}
        height={400}
        fixedItemHeight={30}
        initialItemCount={Math.min(rows.length, 20)}
        components={{
          Table: (props) => (
            <table
              data-testid="virtuoso-table"
              role="table"
              aria-label="Query results"
              {...props}
            />
          ),
          TableHead: (props) => (
            <thead data-testid="virtuoso-thead" {...props} />
          ),
          TableBody: (props) => (
            <tbody data-testid="virtuoso-item-list" {...props} />
          ),
        }}
        fixedHeaderContent={() => (
          <tr role="row" data-testid="table-header">
            {columns.map(col => {
              const isSorted = sortKey === col.key;
              const dir = nextSortDir(sortDir, isSorted);

              return (
                <th
                  key={col.key}
                  role="columnheader"
                  aria-sort={
                    isSorted
                      ? sortDir === 'Asc'
                        ? 'ascending'
                        : 'descending'
                      : 'none'
                  }
                  className="border-b bg-muted/50 px-2 py-1 text-left font-medium"
                  style={{ width: col.width, minWidth: col.width, maxWidth: col.width }}
                >
                  <span className="inline-flex items-center gap-1">
                    <span>{col.header}</span>
                    {col.sortable && (
                      <button
                        type="button"
                        data-testid={`sort-${col.key}`}
                        aria-label={`Sort by ${col.header}`}
                        onClick={() => handleSort(col.key, dir)}
                        className="ml-1 rounded px-1 text-xs hover:bg-muted-foreground/20"
                      >
                        {isSorted ? (sortDir === 'Asc' ? '↑' : '↓') : '↕'}
                      </button>
                    )}
                  </span>
                </th>
              );
            })}
          </tr>
        )}
        itemContent={(index, row) => (
          <tr
            data-testid={`table-row-${index}`}
            role="row"
            aria-label={`Row ${index + 1}`}
            className="border-b border-muted hover:bg-muted/30"
          >
            {columns.map(col => (
              <td
                key={col.key}
                role="cell"
                data-testid={`cell-${col.key}-${index}`}
                className="px-2 py-1"
                style={{
                  width: col.width,
                  minWidth: col.width,
                  maxWidth: col.width,
                }}
              >
                {col.render
                  ? col.render(row[col.key], row)
                  : DefaultCell({ value: row[col.key] })}
              </td>
            ))}
          </tr>
        )}
      />
    </div>
  );
}
