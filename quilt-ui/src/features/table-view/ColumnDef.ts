/**
 * ColumnDef types — F17 TableView column configuration.
 *
 * Defines the shape of a table column and its rendering behavior.
 * Used by `react-virtuoso` TableVirtuoso for virtualized rendering.
 */

import type { SortDirection } from '../../shared/types/queryAst';

// ─── ColumnDef ─────────────────────────────────────────────────────────────────

/** A single column in a TableView. */
export interface ColumnDef {
  /**
   * Property key this column displays.
   * Used as the data lookup key for cell values.
   */
  key: string;
  /** Column header text displayed in the table header row. */
  header: string;
  /**
   * Column width in pixels.
   * Used by react-virtuoso's fixedWidthData property.
   */
  width: number;
  /**
   * Whether this column is sortable.
   * When true, clicking the header triggers a re-execute with SortBy.
   * @default false
   */
  sortable?: boolean;
  /**
   * Optional custom cell renderer.
   * Receives the raw cell value and returns a React node.
   * If not provided, a default renderer is used based on value type.
   */
  render?: (value: unknown, row: Record<string, unknown>) => React.ReactNode;
}

/** Default column width when not specified. */
export const DEFAULT_COLUMN_WIDTH = 150;

/** Min column width enforced by TableView. */
export const MIN_COLUMN_WIDTH = 60;

/** Max column width enforced by TableView. */
export const MAX_COLUMN_WIDTH = 500;
