// ──── Enums ─────────────────────────────────────────────────────

export type TaskMarker = "Now" | "Later" | "Todo" | "Doing" | "Done" | "Cancelled" | "Waiting";
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

/**
 * Response body for `GET /api/v1/user/tour-state` and the
 * `POST /api/v1/user/tour-state/dismiss` happy-path response
 * (B of `quilt-fase4-cross-device-tour`).
 *
 * `dismissed` is the alphabetically-sorted list of tour-name slugs
 * (`"welcome"`, `"cognitive"`, `"mcp"`, ...) that the current user
 * has dismissed on at least one device. The server treats the
 * `Authorization: Bearer <key>` token as the user identifier for V1.
 */
export interface TourStateResponse {
  dismissed: string[];
}

/** Body of `POST /api/v1/user/tour-state/dismiss`. */
export interface DismissTourRequest {
  /** Short slug for the tour to dismiss (e.g. `"welcome"`). */
  tour: string;
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
  /** ISO-8601 string (YYYY-MM-DDTHH:MM:SSZ). Written by the /Scheduled slash command or InlinePropertyBadges. */
  scheduled?: string | null;
  /** ISO-8601 string (YYYY-MM-DDTHH:MM:SSZ). Written by the /Deadline slash command or InlinePropertyBadges. */
  deadline?: string | null;
  /** ISO-8601 string. Set automatically when marker becomes Done/Cancelled. */
  logbook?: string | null;
  /** ISO-8601 string. Set when marker becomes Doing/Now (P2). */
  startTime?: string | null;
  /** ISO-8601 string. Next occurrence for recurring tasks (P2). */
  repeated?: string | null;
  /** ISO-8601 string. Set when marker becomes Done. */
  completedAt?: string | null;
  /** ISO-8601 string. Set when marker becomes Cancelled. */
  cancelledAt?: string | null;
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
  /** ISO-8601 string (YYYY-MM-DDTHH:MM:SSZ) to set; null to clear; absent to leave unchanged. */
  scheduled?: string | null;
  /** ISO-8601 string (YYYY-MM-DDTHH:MM:SSZ) to set; null to clear; absent to leave unchanged. */
  deadline?: string | null;
}

// ──── Settings ──────────────────────────────────────────────────

export interface UserSettings {
  timezone: string;
  journalFormat: string;
  startOfWeek: number;
  preferredFormat: string;
  /** Whether to show daily aggregation sections on journal pages (default: false) */
  journalAggregate?: boolean;
}

export interface UpdateSettingsRequest {
  timezone?: string;
  journalFormat?: string;
  startOfWeek?: number;
  preferredFormat?: string;
  journalAggregate?: boolean;
}

export interface DateFormatOption {
  pattern: string;
  example: string;
}

// ──── Graph Space ─────────────────────────────────────────────────

export interface GraphSpace {
  name: string;
  description: string;
  version: string;
}

export interface UpdateGraphSpaceRequest {
  name?: string;
  description?: string;
}

// ──── Backlinks ─────────────────────────────────────────────────

export interface Backlink {
  sourceBlockId: string;
  sourcePageName: string;
  contentPreview: string;
  /**
   * The snippet shown in the Backlinks panel for this reference.
   * Q028 (Editable Backlinks): the server returns the user-edited
   * override when one is set, otherwise the source block's content
   * snippet. The panel uses this field for display and as the
   * starting value when the user opens the inline editor.
   */
  context: string;
}

// ──── Search results ─────────────────────────────────────────────

/**
 * Shape returned by `GET /api/v1/blocks/search?q=...` and
 * `GET /api/v1/search?q=...`. Mirrors the Rust `SearchResultDto`
 * (`crates/quilt-server/src/handlers/search.rs`). G3 of the wikilinks
 * audit wires the search modal to this endpoint so users can find
 * blocks by content, not just by page name.
 *
 * S1-04: `properties` carries the structured property bag from the
 * `blocks.properties` BLOB. The frontend's `blockMatchesFilter` uses
 * this for filter matching instead of regex-matching raw content.
 */
export interface SearchResult {
  blockId: string;
  pageId: string;
  pageName: string;
  content: string;
  snippet: string;
  score: number;
  properties?: BlockProperty[];
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

// ──── Morning Briefing ────────────────────────────────────────────────

/** An item in today's agenda — a block from today's journal page. */
export interface AgendaItem {
  blockId: string;
  contentPreview: string;
  pageName: string;
  hasChildren: boolean;
  updatedAt: string;
}

/** A block that has decayed — not updated in a while and may need attention. */
export interface DecayAlert {
  blockId: string;
  contentPreview: string;
  pageName: string;
  daysSinceUpdate: number;
  severity: 'low' | 'medium' | 'high';
  reason: string;
}

/** A serendipitous connection discovered between two blocks. */
export interface SerendipityHighlight {
  blockAId: string;
  blockBId: string;
  blockAPreview: string;
  blockBPreview: string;
  explanation: string;
  confidence: number;
}

/** The complete morning briefing response. */
export interface MorningBriefingDto {
  agendaItems: AgendaItem[];
  decayAlerts: DecayAlert[];
  serendipityHighlights: SerendipityHighlight[];
  generatedAt: string;
  daysSinceLastJournal: number;
}

// ──── Decay Monitor (CG-7) ─────────────────────────────────────────────

/** Per-severity counts of decay alerts (mirrors the Rust DTO). */
export interface SeverityCounts {
  low: number;
  medium: number;
  high: number;
}

/** Response body for `GET /api/v1/cognitive/decay`. */
export interface DecayMonitorDto {
  alerts: DecayAlert[];
  totalAlerts: number;
  countsBySeverity: SeverityCounts;
  generatedAt: string;
}

// ──── Weekly Review (CG-7) ─────────────────────────────────────────────

/** Direction of the decay trend over the last two weeks. */
export type DecayTrend = 'worsening' | 'improving' | 'stable';

/** Response body for `GET /api/v1/cognitive/weekly-review`. */
export interface WeeklyReviewDto {
  weekStart: string;
  weekEnd: string;
  blocksCreated: number;
  blocksUpdated: number;
  tasksCompleted: number;
  decayTrend: DecayTrend;
  decayDelta: number;
  journalDays: number;
  suggestions: string[];
  generatedAt: string;
}

// ─── Serendipity Monitor (CG-3) ─────────────────────────────────────────────

/** Response body for `GET /api/v1/cognitive/serendipity`. */
export interface SerendipityResponseDto {
  highlights: SerendipityHighlight[];
  total: number;
  generatedAt: string;
}

// ─── Cognitive Graph (CG-2) ─────────────────────────────────────────────────

/** A node in the cognitive graph — represents a block. */
export interface CognitiveGraphNode {
  id: string;
  blockId: string;
  pageId: string;
  pageName: string;
  contentPreview: string;
  influenceScore: number;
  isFrontier: boolean;
  isGap: boolean;
  clusterId: string | null;
}

/** An edge in the cognitive graph — represents a reference between blocks. */
export interface CognitiveGraphEdge {
  from: string;
  to: string;
}

/** A detected knowledge cluster. */
export interface CognitiveGraphCluster {
  id: string;
  blockIds: string[];
  theme: string | null;
  coherenceScore: number;
}

/** Response body for `GET /api/v1/cognitive/graph`. */
export interface CognitiveGraphDto {
  nodes: CognitiveGraphNode[];
  edges: CognitiveGraphEdge[];
  clusters: CognitiveGraphCluster[];
  frontierNodes: string[];
  gapNodes: string[];
  generatedAt: string;
}

// ─── Agent Room (CG-5) ───────────────────────────────────────────────────────
//
// The V1 Agent Room surface. The string set of `status` matches the one
// `AgentRunRenderer` already renders — `Queued`, `Running`, `Completed`,
// `Failed`, `Cancelled`. New agent types (only `decay-annotator` in V1)
// are added by registering an `AgentExecutor` in
// `quilt-analysis::agent_room::registry::AgentRegistry` server-side; the
// wire format is unchanged.

/** Lifecycle state of an agent run. */
export type AgentStatus =
  | 'Queued'
  | 'Running'
  | 'Completed'
  | 'Failed'
  | 'Cancelled';

/** A single agent run. The shape matches the Rust `AgentDto`. */
export interface AgentDto {
  /** UUID of the underlying AgentRun block. */
  id: string;
  /** Agent type id, e.g. `decay-annotator` in V1. */
  agentType: string;
  /** Informational model label (no LLM in V1, per ADR-0001). */
  model?: string | null;
  /** Current lifecycle state. */
  status: AgentStatus;
  /** Optional context page the agent was scoped to. */
  contextPage?: string | null;
  /** One-line summary set when the agent reaches `Completed`. */
  summary?: string | null;
  /** Number of blocks this agent has written to the graph. */
  blocksModified: number;
  /** When the worker started. `null` while `Queued`. */
  startedAt?: string | null;
  /** When the run reached a terminal state. */
  completedAt?: string | null;
  /** Error message; populated only when `status === 'Failed'`. */
  error?: string | null;
}

/** Response body for `GET /api/v1/agents`. */
export interface AgentListResponse {
  agents: AgentDto[];
  /** Full registry size regardless of the `?limit=` filter. */
  total: number;
}

/** Request body for `POST /api/v1/agents`. Optional fields default to `null`. */
export interface SpawnAgentRequest {
  agentType: string;
  contextPage?: string;
  model?: string;
  /** Accepted for forward compatibility; ignored in V1. */
  queueMode?: 'sequential' | 'parallel';
}
