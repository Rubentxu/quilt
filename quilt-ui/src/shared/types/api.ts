// ──── Enums ─────────────────────────────────────────────────────

export type TaskMarker = "Now" | "Later" | "Todo" | "Done" | "Cancelled";
export type Priority = "A" | "B" | "C";
export type BlockType = "paragraph" | "heading1" | "heading2" | "heading3" | "bullet" | "numbered" | "todo" | "quote" | "code" | "divider" | "image";
export type AppErrorCode = "NOT_FOUND" | "BAD_REQUEST" | "INTERNAL_ERROR";

// ──── Templates (ADR-0007) ───────────────────────────────────

/** Card shape declared by a template page via `card-shape::`. */
export type CardShape = "reference" | "content" | "inline";

/** Summary of one template page — returned by GET /api/v1/templates. */
export interface TemplateSummary {
  /** Short name (e.g., "reference" for `template/reference`). */
  name: string;
  /** Full page name including the `template/` prefix. */
  full_name: string;
  /** Total blocks on the template page. */
  block_count: number;
  /** The card-shape value, defaulting to "inline" if missing. */
  card_shape: CardShape | string;
  /** The `icon::` value, if declared. */
  icon?: string | null;
  /** The `cssclass::` value, if declared. */
  cssclass?: string | null;
}

/** Schema of one template — returned by GET /api/v1/templates/:name/schema. */
export interface TemplateSchema {
  name: string;
  full_name: string;
  card_shape: CardShape | string;
  icon?: string | null;
  cssclass?: string | null;
  block_count: number;
  /** Union of all properties declared across the template's blocks. */
  properties: TemplateProperty[];
}

/** One property of a template's contract. */
export interface TemplateProperty {
  key: string;
  value: string;
  /** JSON-ish type: "string" | "boolean" | "integer" | "float" | "date" | "ref" | "array" */
  type: string;
  /** Canonical property type: "Text" | "Number" | "Date" | "DateTime" | "Url" | "Checkbox" | "Node" | undefined */
  property_type?: string;
}

// ──── Shared ────────────────────────────────────────────────────

export interface ApiError {
  error: string;
  code: AppErrorCode;
}

// ──── Pages ─────────────────────────────────────────────────────

export interface Page {
  id: string;
  name: string;
  title: string | null;
  journal: boolean;
  journalDay: number | null;
  createdAt: string;
}

// ──── Sidebar Recents ─────────────────────────────

/**
 * One entry in the recent-pages sidebar list (PR 2 of
 * `quilt-fase1-sidebar-mcp-templates`). Tracked client-side and
 * persisted to `localStorage` under `STORAGE_KEYS.RECENTS`.
 *
 * - `name` is the canonical page name (case-preserving).
 * - `url` is the route (e.g. `/page/foo`) — captured at visit time.
 * - `visitedAt` is a Unix timestamp in milliseconds, refreshed on
 *   every re-visit (move-to-top).
 */
export interface RecentPage {
  name: string;
  url: string;
  visitedAt: number;
}

export interface CreatePageRequest {
  name: string;
  title?: string;
  /** Mark this page as a journal/daily-note page */
  isJournal?: boolean;
  /** Journal day in YYYYMMDD format (required when isJournal is true) */
  journalDay?: string;
}

// ──── Templates (ADR-0003) ──────────────────────────────────────

/** Body of `POST /api/v1/pages/from-template`. */
export interface CreatePageFromTemplateRequest {
  /** Name of the template page (must start with `template/`). */
  templateName: string;
  /** Name of the new page to create from the template. */
  pageName: string;
  /** Optional display title for the new page. Defaults to `pageName`. */
  title?: string;
  /**
   * Optional `{{var}}` / `${var}` substitutions, applied on top of
   * built-in variables (`title`, `name`, `date`).
   */
  variables?: Record<string, string>;
}

/** Response of `POST /api/v1/pages/from-template`. */
export interface CreatePageFromTemplateResponse {
  page: Page;
  /** Number of blocks cloned from the template into the new page. */
  blocksCreated: number;
}

// ──── Block Properties ──────────────────────────────────────────

export interface BlockProperty {
  key: string;
  value: string | number | boolean | null;
  type: 'string' | 'number' | 'boolean' | 'date' | 'select' | 'page_ref';
}

// ──── Blocks ────────────────────────────────────────────────────

export interface Block {
  id: string;
  pageId: string;
  pageName: string | null;
  content: string;
  blockType: BlockType;
  marker: TaskMarker | null;
  priority: Priority | null;
  parentId: string | null;
  order: number;
  level: number;
  collapsed: boolean;
  properties?: BlockProperty[];
  createdAt: string;
  updatedAt: string;
}

export interface CreateBlockRequest {
  pageName: string;
  content: string;
  parentId?: string | null;
  precedingBlockId?: string | null;
  /**
   * Initial properties to attach to the block. Each value is a JSON value
   * (string, number, boolean, array). Used by features like comments
   * (`{ type: "comment", resolved: "false", created_by: "..." }`).
   */
  properties?: Record<string, unknown>;
  /**
   * Convenience field for the `created_by` convention
   * (ADR-0003: `user::<name>` for humans, `agent::<name>` for AI).
   * The server will set this as a property if not already present in
   * `properties`. Sending it explicitly in `properties` wins.
   */
  createdBy?: string;
}

export interface UpdateBlockRequest {
  content?: string;
  blockType?: BlockType;
  marker?: TaskMarker | null;
  priority?: Priority | null;
  parentId?: string | null;
  order?: number;
  level?: number;
  collapsed?: boolean;
}

// ──── Settings ──────────────────────────────────────────────────

export interface UserSettings {
  timezone: string;
  journalFormat: string;
  startOfWeek: number;
  preferredFormat: string;
}

export interface UpdateSettingsRequest {
  timezone?: string;
  journalFormat?: string;
  startOfWeek?: number;
  preferredFormat?: string;
}

export interface DateFormatOption {
  pattern: string;
  example: string;
}

// ──── Backlinks ─────────────────────────────────────────────────

export interface Backlink {
  sourceBlockId: string;
  sourcePageName: string;
  contentPreview: string;
}

// ──── Search results ─────────────────────────────────────────────

/**
 * Shape returned by `GET /api/v1/blocks/search?q=...` and
 * `GET /api/v1/search?q=...`. Mirrors the Rust `SearchResultDto`
 * (`crates/quilt-server/src/handlers/search.rs`). G3 of the wikilinks
 * audit wires the search modal to this endpoint so users can find
 * blocks by content, not just by page name.
 */
export interface SearchResult {
  blockId: string;
  pageId: string;
  pageName: string;
  content: string;
  snippet: string;
  score: number;
}

// ──── OutlinerCommand (WASM history bridge) ───────────────────────
//
// Mirrors `crate::outliner::history::OutlinerCommand` (Rust). The
// `#[serde(tag = "type", rename_all = "camelCase")]` attributes on
// the Rust enum produce the exact JSON shape this type describes.
// Pass one of these values to `wasm.historyApply(stackId, cmd)` and
// the command will be applied to the WASM-side block list and pushed
// onto the history stack for later undo/redo.

export type OutlinerCommand =
  | {
      type: 'setContent';
      blockId: string;
      before: string;
      after: string;
    }
  | {
      type: 'autocompleteInsert';
      blockId: string;
      before: string;
      after: string;
      trigger: string;
    }
  | {
      type: 'splitBlock';
      blockId: string;
      newBlockId: string;
      firstPart: string;
      secondPart: string;
    }
  | {
      type: 'mergeBlock';
      targetId: string;
      sourceId: string;
      targetBefore: string;
      sourceBefore: string;
    }
  | {
      type: 'indent';
      blockId: string;
      oldParent: string | null;
      oldOrder: number;
      newParent: string | null;
      newOrder: number;
    }
  | {
      type: 'outdent';
      blockId: string;
      oldParent: string | null;
      oldOrder: number;
      newParent: string | null;
      newOrder: number;
    }
  | {
      type: 'moveBlock';
      blockId: string;
      oldParent: string | null;
      oldOrder: number;
      newParent: string | null;
      newOrder: number;
    };

// ──── Evidence Contract v1 (ADR-0008) ──────────────────────────────────
//
// Every MCP tool/resource response carries a `_meta.evidence` envelope
// (server-level, additive — pre-change wire format is byte-identical
// when the field is absent). Mirrors the Rust types in
// `crates/quilt-mcp/src/protocol.rs` (Evidence, SourceAuthority,
// MetaEnvelope). All fields are optional since each tool/resource tier
// (rich / sparse / fallback) only populates a subset.

/** G2 source authority — Manual > PropertyTyped > AutoExtracted. */
export type SourceAuthority = 'Manual' | 'PropertyTyped' | 'AutoExtracted';

/**
 * Provenance metadata attached to every MCP tool/resource response.
 * `is_error: true` indicates the handler returned an error; in that
 * case the rest of the fields are at their defaults.
 */
export interface Evidence {
  /** Name of the tool that produced this response (or URI for resources). */
  toolName: string;
  /** Server timestamp at injection time (ISO-8601 RFC-3339). */
  timestamp: string;
  /** True when the handler returned an error envelope. */
  isError: boolean;
  /** Block IDs touched/produced by this tool call. */
  blockIds: string[];
  /** Page name when the tool references a single page. */
  pageName?: string;
  /** Page `updatedAt` (ISO-8601 RFC-3339) when the tool references a single page. */
  pageUpdatedAt?: string;
  /** DSL query AST for `quilt_query` (string form). */
  queryAst?: string;
  /** Matched search terms for `quilt_search`. */
  matchedTerms: string[];
  /** Source authority ranking (G2). None when not derivable. */
  sourceAuthority?: SourceAuthority;
}

/**
 * `_meta` envelope carried by `ToolsCallResult` and `ResourceReadResult`.
 * Reserved at server level — no handler serializes a top-level
 * `evidence` key from its returned string.
 */
export interface MetaEnvelope {
  evidence?: Evidence;
}
