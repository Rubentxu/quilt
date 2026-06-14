/**
 * PropertySchema — persisted as a JSON string in the `schema::` block property
 * on the SOURCE query block (not the view). Single source of truth for all
 * views that reference this data source (OQ-2 confirmed).
 *
 * When `schema::` is absent, the UI falls back to auto-detection from
 * block.properties values (P1 behaviour).
 */

/** The full schema for a query/table's properties. */
export interface PropertySchema {
  /** Ordered list of property definitions. */
  properties: PropertyDef[]
}

/** A single property definition in the schema. */
export interface PropertyDef {
  /** Property key (matches the key in block.properties). */
  key: string

  /** Human-readable display name. Defaults to key if omitted. */
  name?: string

  /** Property data type. */
  type: PropertyType

  /** Type-specific options. */
  options?: PropertyOptions
}

/** Supported property types. */
export type PropertyType =
  | 'text'
  | 'number'
  | 'select'
  | 'multi_select'
  | 'date'
  | 'boolean'
  | 'url'
  | 'person'
  | 'relation'
  | 'file'

/** Type-specific options, discriminated by PropertyType. */
export type PropertyOptions =
  | SelectOptions
  | DateOptions
  | NumberOptions
  | RelationOptions
  | TextOptions

/** Select / multi_select: predefined options with optional colours. */
export interface SelectOptions {
  type: 'select' | 'multi_select'
  options: SelectOption[]
}

export interface SelectOption {
  name: string
  color?: string // CSS colour value or design token
}

/** Date: format and display preferences. */
export interface DateOptions {
  type: 'date'
  /** Display format: 'YYYY-MM-DD', 'relative' (2 days ago), 'MMMM D, YYYY', etc. */
  format?: string
  /** Whether to include time. Default false. */
  includeTime?: boolean
}

/** Number: formatting preferences. */
export interface NumberOptions {
  type: 'number'
  /** Format: 'decimal', 'percent', 'currency', 'compact'. */
  format?: 'decimal' | 'percent' | 'currency' | 'compact'
  /** Currency symbol when format is 'currency'. */
  currency?: string
  /** Decimal places. Default 0. */
  decimals?: number
}

/** Relation: target database/page reference. */
export interface RelationOptions {
  type: 'relation'
  /** Target page/database ID or name for the relation. */
  target: string
  /** Whether the relation is bidirectional. Default false. */
  bidirectional?: boolean
}

/** Text: placeholder text for empty text values. */
export interface TextOptions {
  type: 'text'
  /** Placeholder text shown when the value is empty. */
  placeholder?: string
}

/** Default property type when no schema is present. */
export const DEFAULT_PROPERTY_TYPE: PropertyType = 'text'
