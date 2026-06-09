/**
 * SavedViewBlock — ADR-DRAFT-saved-view-block-role
 *
 * Renders a SavedView: a block with `type:: view` that composes a
 * reference to a Query block via `data-source::` and selects a
 * renderer via `view-type::` (table, kanban, list, graph, cards,
 * calendar, timeline).
 *
 * Per the ADR, the SavedView is a *role* carried on a regular Block
 * (not a new domain entity). All the data — the query DSL, the
 * grouping, the sort — lives on the source block. This component
 * is the renderer dispatcher.
 *
 * ─── Property contract ──────────────────────────────────────────
 *
 *   | Property       | Required | Purpose                          |
 *   |----------------|----------|----------------------------------|
 *   | type::         | yes      | "view" — the role marker         |
 *   | view-type::    | yes      | table | kanban | list | ...       |
 *   | data-source::  | yes      | UUID of the source query block   |
 *   | view-name::    | no       | Display label                    |
 *   | group-by::     | no       | Property key for kanban grouping |
 *
 * ─── Source lookup (frontend-only) ───────────────────────────────
 *
 * Per the task constraint "no backend changes needed", the source
 * block is looked up in the `allBlocks` array that the parent
 * BlockRow already provides. This means SavedView and its source
 * Query must live on the same page in V1. Cross-page view references
 * fall through to the error state below.
 *
 * If a future ADR lifts that constraint, the lookup is a single
 * substitution (`api.getBlock(uuid)`) — the rest of the component
 * stays the same.
 *
 * ─── Renderer dispatch ──────────────────────────────────────────
 *
 *   view-type    → renderer
 *   ───────────  ───────────────────────────────────────────────
 *   "kanban"     → <KanbanBoard>   (existing F22 component)
 *   "table"      → <TableView>     (existing F17 component)
 *   "list"       → <PlaceholderList>
 *   "graph"      → <PlaceholderGraph>
 *   "cards"      → <PlaceholderCards>
 *   "calendar"   → <PlaceholderCalendar>
 *   "timeline"   → <PlaceholderTimeline>
 *   anything else → error state with the unrecognised value
 *
 * Placeholders keep the dispatcher testable while leaving the real
 * renderers for their own follow-up ADRs. The placeholder contract
 * is: a single `data-testid="saved-view-<viewtype>"` div.
 */

import type { Block, BlockProperty } from '@shared/types/api'
import { TableView } from '@features/table-view/TableView'
import { KanbanBoard } from '@features/kanban/KanbanBoard'
import type { ColumnDef } from '@features/table-view/ColumnDef'

// ──── Public types ─────────────────────────────────────────────

/** Recognised view-type:: values. Anything else triggers the error state. */
export const VIEW_TYPES = [
  'table',
  'kanban',
  'calendar',
  'list',
  'graph',
  'cards',
  'timeline',
] as const

export type ViewType = (typeof VIEW_TYPES)[number]

/** Default kanban grouping key when the block has no `group-by::`. */
const DEFAULT_KANBAN_GROUP_BY = 'status'

/** Default table column shape for V1. */
const DEFAULT_TABLE_COLUMNS: ColumnDef[] = [
  { key: 'content', header: 'Block', width: 320 },
  { key: 'type', header: 'Type', width: 120 },
]

export interface SavedViewBlockProps {
  /** The view block itself (carries view-type::, data-source::, view-name::). */
  block: Block
  /**
   * All blocks on the current page. The source query block must be
   * present here for the view to render. Cross-page views fall
   * through to the error state (see file header).
   */
  allBlocks: Block[]
}

// ──── Property readers ────────────────────────────────────────
//
// `block.properties` is `BlockProperty[]` (per @shared/types/api).
// We use small helpers instead of `Array.prototype.find` inline so
// the dispatcher stays readable and the property-key convention is
// documented in one place.

/** Read a string property, or null if absent/non-string. */
function readStringProperty(
  block: Block,
  key: string,
): string | null {
  if (!block.properties) return null
  const prop = block.properties.find((p) => p.key === key)
  if (!prop || prop.value == null) return null
  return String(prop.value)
}

/** Read a property as a string with a fallback. */
function readStringWithDefault(
  block: Block,
  key: string,
  fallback: string,
): string {
  return readStringProperty(block, key) ?? fallback
}

/** Type guard: is the given string a recognised ViewType? */
function isViewType(value: string | null): value is ViewType {
  return value !== null && (VIEW_TYPES as readonly string[]).includes(value)
}

// ──── Component ────────────────────────────────────────────────

export function SavedViewBlock({ block, allBlocks }: SavedViewBlockProps) {
  const viewTypeRaw = readStringProperty(block, 'view-type')
  const dataSource = readStringProperty(block, 'data-source')
  const viewName = readStringWithDefault(block, 'view-name', 'Untitled view')
  const groupBy = readStringWithDefault(
    block,
    'group-by',
    DEFAULT_KANBAN_GROUP_BY,
  )

  // ── Error: data-source:: missing ──────────────────────────
  if (!dataSource) {
    return (
      <SavedViewError
        reason="This SavedView is missing the required `data-source::` property pointing to a query block."
      />
    )
  }

  // ── Error: view-type:: missing or unrecognised ─────────────
  if (!isViewType(viewTypeRaw)) {
    return (
      <SavedViewError
        reason={
          viewTypeRaw === null
            ? 'This SavedView is missing the required `view-type::` property.'
            : `Unknown \`view-type:: "${viewTypeRaw}"`.concat(
                ' — expected one of: ',
                VIEW_TYPES.join(', '),
                '.',
              )
        }
      />
    )
  }

  // ── Look up the source block in the parent-provided allBlocks ─
  const source = allBlocks.find((b) => b.id === dataSource)
  if (!source) {
    return (
      <SavedViewError
        reason={
          `Source query block "${dataSource}" was not found on this page. ` +
          'In V1 the source Query block must live on the same page as the view.'
        }
      />
    )
  }

  // ── Dispatch to the renderer matching view-type:: ──────────
  return (
    <div
      data-testid="saved-view-block"
      style={{
        // The view occupies the full content width of the block row;
        // BlockRow's flex container handles the chrome around us.
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-2)',
        padding: 'var(--space-2) 0',
      }}
    >
      <div
        data-testid="saved-view-name"
        style={{
          fontSize: '12px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.04em',
        }}
      >
        {viewName}
      </div>

      <ViewTypeDispatcher
        viewType={viewTypeRaw}
        source={source}
        groupBy={groupBy}
      />
    </div>
  )
}

// ──── Renderer dispatcher ─────────────────────────────────────
//
// One small switch (not a Map<ViewType, Component>) because the
// per-view-type wrappers need different props (KanbanBoard wants
// `blocks` + `propertyKey`, TableView wants `columns` + `rows`).
// A map-of-components would either need a uniform shape or a
// discriminated wrapper for each — neither buys us anything here.

function ViewTypeDispatcher({
  viewType,
  source,
  groupBy,
}: {
  viewType: ViewType
  source: Block
  groupBy: string
}) {
  switch (viewType) {
    case 'kanban':
      return (
        <div data-testid="saved-view-kanban">
          <KanbanBoard
            // V1 contract: the source block is the only card on the
            // board. Future revisions (V2) will pass the DSL results
            // once the source is executed by the dispatcher.
            blocks={[source]}
            propertyKey={groupBy}
            onPropertyChange={() => {
              // V1: no-op. The kanban board expects a handler but
              // the source is read-only from the view's perspective.
            }}
          />
        </div>
      )

    case 'table':
      return (
        <div data-testid="saved-view-table">
          <TableView
            columns={DEFAULT_TABLE_COLUMNS}
            rows={[
              {
                content: source.content,
                type: readStringWithDefault(source, 'type', 'paragraph'),
              },
            ]}
          />
        </div>
      )

    case 'list':
      return (
        <PlaceholderList
          testid="saved-view-list"
          sourceContent={source.content}
        />
      )

    case 'graph':
      return <PlaceholderGraph testid="saved-view-graph" />

    case 'cards':
      return (
        <PlaceholderCards
          testid="saved-view-cards"
          sourceContent={source.content}
        />
      )

    case 'calendar':
      return <PlaceholderCalendar testid="saved-view-calendar" />

    case 'timeline':
      return <PlaceholderTimeline testid="saved-view-timeline" />
  }
}

// ──── Placeholders ────────────────────────────────────────────
//
// Each placeholder is a minimal div with the matching testid. The
// real renderers for list / graph / cards / calendar / timeline
// are out of scope for this ADR — they will land in their own
// follow-ups. The placeholders keep the dispatcher testable today
// and give the user a clear "this view-type is recognised but its
// renderer is not implemented yet" signal.

function PlaceholderList({
  testid,
  sourceContent,
}: {
  testid: string
  sourceContent: string
}) {
  return (
    <div
      data-testid={testid}
      style={placeholderStyle}
      aria-label="List view (placeholder)"
    >
      <span aria-hidden="true">📋</span>
      <span>
        List view — source: <em>{sourceContent || '(empty)'}</em>
      </span>
    </div>
  )
}

function PlaceholderGraph({ testid }: { testid: string }) {
  return (
    <div data-testid={testid} style={placeholderStyle} aria-label="Graph view (placeholder)">
      <span aria-hidden="true">🕸️</span>
      <span>Graph view (renderer pending)</span>
    </div>
  )
}

function PlaceholderCards({
  testid,
  sourceContent,
}: {
  testid: string
  sourceContent: string
}) {
  return (
    <div data-testid={testid} style={placeholderStyle} aria-label="Cards view (placeholder)">
      <span aria-hidden="true">🗂️</span>
      <span>
        Cards view — source: <em>{sourceContent || '(empty)'}</em>
      </span>
    </div>
  )
}

function PlaceholderCalendar({ testid }: { testid: string }) {
  return (
    <div data-testid={testid} style={placeholderStyle} aria-label="Calendar view (placeholder)">
      <span aria-hidden="true">📅</span>
      <span>Calendar view (renderer pending)</span>
    </div>
  )
}

function PlaceholderTimeline({ testid }: { testid: string }) {
  return (
    <div data-testid={testid} style={placeholderStyle} aria-label="Timeline view (placeholder)">
      <span aria-hidden="true">⏳</span>
      <span>Timeline view (renderer pending)</span>
    </div>
  )
}

// ──── Error state ─────────────────────────────────────────────

function SavedViewError({ reason }: { reason: string }) {
  return (
    <div
      data-testid="saved-view-block"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-1)',
        padding: 'var(--space-2) 0',
      }}
    >
      <div
        data-testid="saved-view-name"
        style={{
          fontSize: '12px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.04em',
        }}
      >
        Saved view
      </div>
      <div
        data-testid="saved-view-error"
        role="alert"
        style={{
          padding: 'var(--space-2) var(--space-3)',
          background: 'var(--color-danger-subtle, rgba(220, 38, 38, 0.08))',
          color: 'var(--color-danger, #dc2626)',
          borderRadius: 'var(--radius-sm)',
          fontSize: '13px',
        }}
      >
        {reason}
      </div>
    </div>
  )
}

// ──── Shared placeholder style ────────────────────────────────

const placeholderStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--space-2)',
  padding: 'var(--space-3)',
  background: 'var(--color-surface-subtle)',
  border: '1px dashed var(--color-border)',
  borderRadius: 'var(--radius-sm)',
  color: 'var(--color-text-muted)',
  fontSize: '13px',
  minHeight: '64px',
}

// Re-export BlockProperty for tests that build fixture blocks.
// (The consumers that need the type already import it directly
// from @shared/types/api, but the re-export keeps the public
// surface of this module self-describing for any future helper.)
export type { BlockProperty }
