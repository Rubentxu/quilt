/**
 * ViewConfig — persisted as a JSON string in the `view-config::` block property
 * on SavedView blocks. Replaces the legacy `view-name::` and `group-by::` loose
 * properties (Phase 2, Batch 8 will handle the migration).
 *
 * All fields have sensible defaults so V1 blocks without `view-config::` still work.
 */
export interface ViewConfig {
  /** Display layout. Must match the view-type:: property. */
  layout: ViewLayout

  /**
   * Display name for the view (was: view-name:: property).
   * Default: "Untitled view"
   */
  name?: string

  /**
   * Which property columns are visible in this view.
   * Omitted = show all. Empty = hide all (edge case).
   */
  visibility?: ViewVisibility

  /**
   * Ordered sort configuration.
   * Omitted = no sort (natural order).
   */
  sort?: ViewSort[]

  /**
   * Filter configuration for this view.
   * Omitted = no filter (show all rows).
   */
  filter?: ViewFilter

  /**
   * Property key for grouping (kanban columns, table sections).
   * Replaces the legacy `group-by::` property.
   * Omitted = no grouping.
   */
  groupBy?: string

  /**
   * Card-specific display options (gallery/cards layout only).
   * Omitted = defaults (show content as title, no cover).
   */
  cardConfig?: CardConfig
}

/** Recognised view layouts. */
export type ViewLayout =
  | 'table'
  | 'gallery'
  | 'list'
  | 'kanban'
  | 'calendar'
  | 'timeline'
  | 'graph'
  | 'cards'

/** Column visibility map. Key = property key, value = visible. */
export interface ViewVisibility {
  /** Per-property visibility. Property keys NOT listed here default to visible. */
  properties?: Record<string, boolean>

  /**
   * Which property to use as the card/list item title.
   * 'content' = use block.content (default).
   * Otherwise = property key to display as title.
   */
  title?: 'content' | string
}

/** A single sort directive. */
export interface ViewSort {
  /** Property key to sort by, or 'content' for the block text. */
  propertyKey: string

  /** Sort direction. */
  direction: 'asc' | 'desc'
}

/** View-level filter. */
export interface ViewFilter {
  /** Logical operator combining all conditions. */
  operator: 'and' | 'or'

  /** Filter conditions. */
  conditions: FilterCondition[]
}

/** A single filter condition. */
export interface FilterCondition {
  /** Property key to filter on. */
  propertyKey: string

  /**
   * Comparison operator. Mirrors quilt-query PropertyOp with 2 additions
   * (IsEmpty, IsNotEmpty — OQ-1 confirmed).
   */
  operator: FilterOperator

  /** Comparison value. NOT required for IsEmpty / IsNotEmpty. */
  value?: string
}

/** Filter operators — PropertyOp from quilt-query + IsEmpty/IsNotEmpty. */
export type FilterOperator =
  | 'Equals'
  | 'NotEquals'
  | 'Contains'
  | 'GreaterThan'
  | 'LessThan'
  | 'Before'
  | 'After'
  | 'IsEmpty'
  | 'IsNotEmpty'

/** Card-specific view configuration (for gallery/cards layout). */
export interface CardConfig {
  /** Property key to use as the card cover image. */
  cover?: string

  /** Property key to display as subtitle below the title. */
  subtitle?: string

  /** Which properties to show as badges/chips on the card. */
  showProperties?: string[]

  /** Card size preset. */
  size?: 'small' | 'medium' | 'large'
}

/** Default ViewConfig — used when a block has no view-config:: property. */
export const DEFAULT_VIEW_CONFIG: ViewConfig = {
  layout: 'table',
  name: 'Untitled view',
}
