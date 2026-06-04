/**
 * SchemaPack TypeScript types — mirrors Rust `quilt_application::templates::schema_pack`.
 *
 * G6: Template metadata stored as JSON in the `schema-pack::` string property.
 * V1 fields: card_shape, icon, cssclass, link_verbs, default_properties, display_hints.
 */

// ─── DisplayFormat ──────────────────────────────────────────────────────────────

/** Display format hint for a property value. */
export type DisplayFormat = 'raw' | 'bold' | 'italic' | 'code';

// ─── DisplayHint ───────────────────────────────────────────────────────────────

/** Per-property display configuration. */
export interface DisplayHint {
  /**
   * How to render this property value.
   * @default "raw"
   */
  format?: DisplayFormat;
  /**
   * Whether to hide this property in the UI.
   * @default false
   */
  hidden?: boolean;
  /**
   * Display order (lower = earlier).
   * @default 0
   */
  order?: number;
}

// ─── DefaultProperty ───────────────────────────────────────────────────────────

/**
 * One default property declaration in a schema pack.
 * Applied when creating a new block from a template.
 */
export interface DefaultProperty {
  /** Property key (e.g., "status", "priority"). */
  key: string;
  /**
   * JSON-ish value type.
   * One of: "string" | "boolean" | "number" | "integer" | "float" | "date" | "array" | "object"
   */
  value_type: string;
  /** Default value as a string (server parses based on value_type). */
  default: string;
}

// ─── SchemaPack ───────────────────────────────────────────────────────────────

/**
 * Schema pack — template metadata as structured JSON.
 *
 * Stored as a single-line JSON string in the `schema-pack::` property
 * on a template page.
 *
 * V1 field set:
 * - card_shape: Card shape override ("reference", "content", "inline")
 * - icon: Icon emoji or text
 * - cssclass: CSS class(es) for custom styling
 * - link_verbs: Link verbs for reference-style cards (e.g., "see also", "references")
 * - default_properties: Default property values to apply when creating a new block
 * - display_hints: Per-property display configuration
 */
export interface SchemaPack {
  /**
   * Card shape override.
   * @default ""
   */
  card_shape?: string;
  /**
   * Icon emoji or text.
   * @default null
   */
  icon?: string | null;
  /**
   * CSS class(es) for custom styling.
   * @default null
   */
  cssclass?: string | null;
  /**
   * Link verbs for reference-style cards.
   * @default []
   */
  link_verbs?: string[];
  /**
   * Default property values to apply when creating a new block.
   * @default []
   */
  default_properties?: DefaultProperty[];
  /**
   * Per-property display configuration.
   * @default {}
   */
  display_hints?: Record<string, DisplayHint>;
}

// ─── SchemaPackApiResponse ────────────────────────────────────────────────────

/** Shape returned by GET /api/v1/templates/:name/schema-pack. */
export interface SchemaPackApiResponse {
  /** The parsed schema pack, or null if the template has no schema-pack property. */
  schema_pack: SchemaPack | null;
}
