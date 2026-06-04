/**
 * QueryBuilder — F17 query builder container.
 *
 * Orchestrates FilterChipGroup + TableView + executeQuery into a
 * cohesive query experience:
 *
 * - FilterChipGroup manages the filter chip list (add/remove/validate)
 * - TableVirtuoso displays the result set (virtualized, sortable)
 * - buildQueryAst converts chips → QueryAst
 * - SortBy wraps the AST when a column header is clicked
 * - AbortController cancels any in-flight request before starting a new one
 *
 * Layout:
 *   ┌─ FilterChipGroup (above table) ──────────────────────┐
 *   │  [chip] [chip] [+ Add filter]                        │
 *   └──────────────────────────────────────────────────────┘
 *   ┌─ TableView (below) ──────────────────────────────────┐
 *   │  name ↓   status ↕   priority  ← click header = sort │
 *   │  ─────────────────────────────────────────────────   │
 *   │  Alice   active    high                              │
 *   └──────────────────────────────────────────────────────┘
 */

import { useCallback, useRef, useState } from 'react';
import { FilterChipGroup } from '../filter-chips/FilterChipGroup';
import { TableView } from '../table-view/TableView';
import type { ColumnDef } from '../table-view/ColumnDef';
import type { FilterChip } from '@shared/types/filterChip';
import type { QueryAst, QueryResult, SortDirection } from '@shared/types/queryAst';
import { buildQueryAst } from '@shared/utils/buildQueryAst';
import { api } from '@core/api-client';
import { validateChipList } from '@shared/types/filterChip';

export interface QueryBuilderProps {
  /** Column definitions for the TableView (required for sort support). */
  columns: ColumnDef[];
  /** Available property keys for the Add Filter dropdown. */
  availableKeys?: string[];
  /** Default limit for query results. @default 100 */
  defaultLimit?: number;
  /** Called when results are returned (includes elapsed_ms for perf display). */
  onResults?: (result: QueryResult) => void;
  /** Called when an error occurs. */
  onError?: (error: Error) => void;
}

// Default columns when none provided
const DEFAULT_COLUMNS: ColumnDef[] = [
  { key: 'name', header: 'Name', width: 200 },
  { key: 'status', header: 'Status', width: 120, sortable: true },
];

// Default limit
const DEFAULT_LIMIT = 100;

/**
 * QueryBuilder — orchestrates filter chips + query execution + result table.
 */
export function QueryBuilder({
  columns: columnsProp,
  availableKeys,
  defaultLimit = DEFAULT_LIMIT,
  onResults,
  onError,
}: QueryBuilderProps) {
  const columns = columnsProp ?? DEFAULT_COLUMNS;

  // ─── State ─────────────────────────────────────────────────────────────────

  const [chips, setChips] = useState<FilterChip[]>([]);
  const [result, setResult] = useState<QueryResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [executeError, setExecuteError] = useState<Error | null>(null);

  // ─── Abort controller for request cancellation ─────────────────────────────

  const abortControllerRef = useRef<AbortController | null>(null);

  const cancelPrevious = useCallback(() => {
    abortControllerRef.current?.abort();
    abortControllerRef.current = new AbortController();
  }, []);

  // ─── Query execution ───────────────────────────────────────────────────────

  /**
   * Execute a query AST with an optional sort override.
   * Cancels any in-flight request before starting.
   */
  const execute = useCallback(
    async (ast: QueryAst | null, sortKey?: string, sortDir?: SortDirection) => {
      cancelPrevious();
      setLoading(true);
      setExecuteError(null);

      const controller = abortControllerRef.current!;

      try {
        // If no AST, return empty result
        if (!ast) {
          setResult({ results: [], total: 0, elapsed_ms: 0 });
          return;
        }

        // Apply SortBy if sort is active
        const effectiveAst: QueryAst = sortKey
          ? { SortBy: { field: sortKey, direction: sortDir ?? 'Asc', inner: ast } }
          : ast;

        const queryResult = await api.executeQuery(
          effectiveAst,
          defaultLimit,
          controller.signal,
        );

        // Only update state if the request wasn't aborted
        if (!controller.signal.aborted) {
          setResult(queryResult);
          onResults?.(queryResult);
        }
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
          // Request was cancelled — this is expected, not an error
          return;
        }
        const error = err instanceof Error ? err : new Error(String(err));
        setExecuteError(error);
        onError?.(error);
      } finally {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      }
    },
    [cancelPrevious, defaultLimit, onResults, onError],
  );

  // ─── Filter apply ───────────────────────────────────────────────────────────

  const handleApply = useCallback(
    (appliedChips: FilterChip[]) => {
      const errors = validateChipList(appliedChips);
      if (Object.keys(errors).length > 0) return;

      const ast = buildQueryAst(appliedChips);
      execute(ast);
    },
    [execute],
  );

  // ─── Sort ──────────────────────────────────────────────────────────────────

  const handleSort = useCallback(
    (sortKey: string, sortDir: SortDirection) => {
      const ast = buildQueryAst(chips);
      execute(ast, sortKey, sortDir);
    },
    [chips, execute],
  );

  // ─── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col gap-4" data-testid="query-builder">
      {/* Filter row */}
      <div data-testid="query-builder-filters">
        <FilterChipGroup
          chips={chips}
          onChange={setChips}
          availableKeys={availableKeys}
          disabled={loading}
          onApply={handleApply}
        />
      </div>

      {/* Error display */}
      {executeError && (
        <div
          data-testid="query-builder-error"
          className="rounded border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive"
          role="alert"
        >
          {executeError.message}
        </div>
      )}

      {/* Loading indicator */}
      {loading && (
        <div data-testid="query-builder-loading" className="text-sm text-muted-foreground">
          Loading…
        </div>
      )}

      {/* Results table */}
      <div data-testid="query-builder-results">
        <TableView
          columns={columns}
          rows={result?.results ?? []}
          onSort={handleSort}
        />
      </div>
    </div>
  );
}
