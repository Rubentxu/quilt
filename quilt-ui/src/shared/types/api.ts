// ──── Enums ─────────────────────────────────────────────────────

export type TaskMarker = "Now" | "Later" | "Todo" | "Done" | "Cancelled";
export type Priority = "A" | "B" | "C";
export type BlockType = "paragraph" | "heading1" | "heading2" | "heading3" | "bullet" | "numbered" | "todo" | "quote" | "code" | "divider" | "image";
export type AppErrorCode = "NOT_FOUND" | "BAD_REQUEST" | "INTERNAL_ERROR";

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
