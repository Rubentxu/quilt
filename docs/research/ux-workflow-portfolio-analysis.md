# Quilt UX Workflow Portfolio Analysis

Deep analysis of Quilt's current user workflows, feature coverage, and improvement opportunities. The goal is to make every feature easier, more useful, and more discoverable while exposing backend capabilities intelligently.

## Executive Summary

Quilt already has a strong backend/domain base: blocks, pages, journals, typed properties, backlinks, search, query execution, templates, MCP tools, and partial cognitive capabilities. The current product gap is not raw capability. The gap is orchestration: users must know which surface to use, where features live, and which syntax/action unlocks each workflow.

The strongest improvement direction is to treat Quilt as a portfolio system:

| User intent | Current burden | Better product behavior |
|---|---|---|
| Capture something | User must choose page/journal/template manually | Default to daily inbox, then suggest structure later |
| Find something | User chooses search, page list, backlinks, graph, or query | One command center ranks pages, blocks, commands, queries, templates, and agents |
| Organize knowledge | User manually edits properties/tags/templates | Contextual suggestions based on similar blocks/pages |
| View structured data | User chooses table/kanban/query route manually | Quilt recommends list/table/kanban/graph/cards based on data shape |
| Use agents | User has passive activity panel only | Agent actions become auditable, reversible graph objects |

The SUNNY algorithm idea is highly reusable for Quilt, not as a CSP solver, but as a general selector of strategies. Quilt can extract a feature vector from the current context, compare it to prior successful contexts, and schedule the best workflow/tool/agent/view.

## Evidence Base

Local code evidence:

| Area | Evidence |
|---|---|
| Routes include pages, journals, graph, table, kanban, query, dashboard | `quilt-ui/src/router.tsx` |
| REST routes expose blocks, pages, search, navigate, settings, templates, query, migration, tour state | `crates/quilt-server/src/routes.rs` |
| API client includes REST calls plus unmounted analysis/schema-pack/SSE assumptions | `quilt-ui/src/core/api-client.ts` |
| Search modal searches pages and FTS block results, with client-side property filter parsing | `quilt-ui/src/features/search/SearchModal.tsx` |
| Query builder has visual chips and table output | `quilt-ui/src/features/query-builder/QueryBuilder.tsx` |
| Query page uses SQL-like text but sends invalid `QueryAst` via `as any` | `quilt-ui/src/pages/QueryPage.tsx` |
| Table/Kanban request all property keys via `getBlockProperties('')`, but backend only exposes block-specific properties | `quilt-ui/src/pages/TablePage.tsx`, `quilt-ui/src/pages/KanbanPage.tsx`, `crates/quilt-server/src/routes.rs` |
| MCP portfolio includes query/search/templates/retrieval/temporal/graph/system tools | `crates/quilt-mcp/src/server.rs` |
| Current docs already identify frontend/cognitive/workflow coverage as partial | `docs/roadmap-gaps/feature-coverage-matrix.md` |
| Logseq alignment contract defines outliner-first behavior | `docs/reversa/logseq-ux-alignment-study.md` |

External pattern evidence:

| Product/pattern family | Reusable insight |
|---|---|
| Logseq/Roam | Daily-first capture, outliner speed, backlinks as first-class navigation |
| Tana | Supertags: type/schema/template/view bundled together |
| Notion | Database views: same data rendered as table/board/calendar/list/gallery |
| Obsidian | Command palette, properties, graph, bases, local-first extensibility |
| Capacities/Anytype | Object types, typed properties, visual organization around semantic objects |
| RemNote | Capture/review loop and portals for reusable embedded context |
| Heptabase | Spatial thinking for research maps and cards |
| SUNNY / sunny-as2 | Portfolio strategy selection by feature vectors and similar historical cases |

SUNNY references:

| Source | Relevant idea |
|---|---|
| `arXiv:1311.3353` | SUNNY computes a solver schedule from a portfolio without learning an explicit model |
| `CP-Unibo/sunny-cp` README | Two phases: pre-solving and solving; feature extraction plus k-NN neighborhood |
| JAIR `sunny-as2` | Generalizes SUNNY to algorithm selection beyond CP |

## Current Workflow Diagnosis

### 1. Navigation And Information Architecture

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| Main routes | `/`, `/page`, `/journal`, `/settings`, `/pages`, `/graph`, `/table`, `/kanban`, `/query`, `/dashboard` | Some routes are invisible from primary sidebar |
| Home | `HomePage` returns `null` | Root path can feel broken |
| Sidebar | Shows journals, pages, graph, favorites, recent pages, templates, recents, optional agent activity | Table, kanban, query, dashboard are not surfaced despite imported icons |
| Tabs | Supports page/journal/graph/all-pages/settings | Structured views are not tab-aware |
| Backlinks panel | Visible right panel for page/journal contexts | Occupies space on routes where backlinks are not relevant |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Make `/` redirect to today's journal or a useful dashboard | Avoids empty landing |
| P0 | Expose Table/Kanban/Query/Dashboard intentionally or remove until ready | Hidden routes create phantom features |
| P1 | Use adaptive navigation: sidebar on desktop, bottom/top compact actions on mobile | Matches Material adaptive navigation and PKM mobile capture needs |
| P1 | Make backlinks contextual: only show on graph-backed pages, with quick collapse and remembered state | Reduces visual noise |

Lateral idea:

| Idea | Description |
|---|---|
| `Work Mode Switcher` | Replace route sprawl with modes: Write, Review, Structure, Explore, Automate. Each mode chooses surfaces and side panels automatically. |

### 2. Capture And Daily Workflow

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| Journal | Daily route and prev/next/today exist | Journal is good foundation |
| Empty journal | Auto-creates first block | Good capture-first behavior |
| New page | Sidebar creates real page via prompt; other flows only navigate | Inconsistent creation semantics |
| Page naming | Lowercases prompt input | May surprise users with title/name behavior |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Standardize page creation through one modal/service | Avoids route-to-nowhere behavior |
| P1 | Add `Quick Capture` command that always appends to today's journal/inbox | Mirrors Logseq, RemNote, Capacities, Tana mobile |
| P1 | Add triage actions on captured blocks: create page, apply type, make task, link to project, schedule date | Captura primero, estructura después |
| P2 | Add natural language date parsing for journals and tasks | `tomorrow`, `next Friday`, `in 2 weeks` is a major PKM accelerator |

Lateral idea:

| Idea | Description |
|---|---|
| `Inbox Triage Rail` | A side rail for untyped daily blocks: each item gets one-click actions to classify, link, convert, or defer. |

### 3. Outliner And Block Editing

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| Block editing | `contentEditable`, click-to-edit, debounce save, slash/autocomplete | Strong base |
| Wikilinks | `[[page]]`, `((block))`, `#tag` autocomplete | `api.listPages()` happens per block row; tags are not fully graph-derived |
| Keyboard | Enter, Backspace, Tab, Shift+Tab, Cmd+B/I/code, task cycling | Good foundation, but help/docs drift exists |
| Delete/cut | Some local delete paths appear not to call API | Risk of data reappearing after refresh |
| Properties panel | Exists, updates on each change | Needs debounce/batch and clearer editing model |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Fix destructive actions to persist through API and support undo | Trust killer if delete reappears |
| P0 | Cache pages/tags/block refs per page/session instead of per `BlockRow` | Prevents N API calls and lag |
| P1 | Promote block zoom and block sidebar open | Core Logseq/Roam navigation pattern |
| P1 | Make properties inline-plus-panel, not either/or | Inline supports outliner speed; panel supports structured editing |
| P2 | Add commandable transforms: paragraph to task, task to project item, block to card, block to query | Turns editor into an action surface |

Lateral idea:

| Idea | Description |
|---|---|
| `Block Shape Detector` | Quilt watches a block's content/properties and suggests “this looks like a task/reference/decision/person/book”. Apply with one keystroke. |

### 4. Search And Command Center

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| Search modal | `Cmd/Ctrl+K`, page + block results, keyboard navigation | Good base |
| Property filters | Parses `status:todo` and regex-matches block content | Does not use typed `Block.properties` reliably |
| Command palette | Search currently finds content, not all actions/tools/templates/views | Underpowered compared to Obsidian/Notion/Tana |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Stop pretending property filters are typed until backend supports them in search/query | Accuracy matters more than clever syntax |
| P1 | Expand `Cmd/Ctrl+K` into universal command center | One door for pages, blocks, commands, views, queries, templates, agents |
| P1 | Add result categories: Pages, Blocks, Commands, Templates, Saved Queries, Agents, Views | Reduces recall burden |
| P2 | Add recent/suggested searches and saved search objects | Search becomes reusable knowledge infrastructure |

Lateral idea:

| Idea | Description |
|---|---|
| `Intent Search` | User types “show open tasks by project” and Quilt proposes Table/Kanban/Query variants instead of just content matches. |

### 5. Query, Table, Kanban, And Structured Views

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| Query backend | Strong Rust DSL and executor | UI does not expose full power safely |
| QueryPage | SQL-like text examples but invalid AST call | Misleading and likely broken |
| QueryBuilder | Chips to AST and TableView | Better UX direction |
| TablePage/KanbanPage | Standalone routes | Property discovery endpoint missing |
| Page-level Kanban | Available if block properties support grouping | Good contextual view concept |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Replace raw QueryPage with either real DSL parser endpoint or hide it | Broken expert mode harms trust |
| P0 | Add backend endpoint for property keys/schema summary | Enables table/kanban/filter UI correctly |
| P1 | Make every query a saved block/object with render mode | Query-as-content is the PKM-native model |
| P1 | Unify `table`, `kanban`, `query` around one `Saved View` model | Same data, multiple renderers |
| P2 | Add view recommendations from data shape | Few statuses -> Kanban, many rows -> Table, dates -> Timeline/Calendar, links -> Graph |

Lateral idea:

| Idea | Description |
|---|---|
| `View Morphing` | Results can switch between list/table/kanban/cards/graph without rebuilding the query. Quilt suggests the best initial renderer. |

### 6. Templates, Cards, And Schemas

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| Template listing | Sidebar uses `api.listTemplates()` | Useful and visible |
| Create from template | Creates `${template.name}-1` | Collision-prone and not user-centered |
| Create template | Modal creates `template/<name>` and seed block | Good start |
| Card rendering | Template-driven shapes exist | Data-driven template schema not fully wired in UI |
| MCP template tools | Include schema, reapply, schema pack | Not fully exposed via REST/UI |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Ask for destination name/title when creating from template | Avoids `template-1` junk |
| P1 | Treat template as contract: properties, blocks, views, suggested agent actions | Makes templates more than copied text |
| P1 | Show preview/diff before applying or reapplying a template | Prevents invisible destructive structure changes |
| P2 | Bind templates to types/tags automatically | Tana/Capacities-style workflow acceleration |

Lateral idea:

| Idea | Description |
|---|---|
| `Template Doctor` | Detect pages/blocks that partially match a template and offer “complete missing fields”, “reapply safely”, or “detach from template”. |

### 7. Backlinks, References, And Graph

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| Backlinks | Right panel lists page backlinks | Useful but not deeply actionable yet |
| Unlinked references | REST endpoint exists for pages | Not surfaced clearly |
| Graph view | Canvas graph over pages/backlinks | Global graph fetches first 50 pages only and lacks filters/lenses |
| Graph theme | Detects `.dark` class while app uses `data-theme` | Dark mode mismatch risk |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Fix graph dark-mode detection | Basic polish/trust |
| P1 | Make backlinks editable/actionable with filters | Backlinks should be workflow, not report |
| P1 | Add local graph from current page/block with depth and relation filters | Global graph is usually decorative; local graph is useful |
| P2 | Add unlinked-reference review queue | High-value Logseq workflow |

Lateral idea:

| Idea | Description |
|---|---|
| `Graph Lens` | A local graph that changes lens: Tasks, People, Decisions, Evidence, Time, Agents. Each lens is a saved query plus visual rules. |

### 8. Cognitive And Agent Workflows

Current state:

| Capability | Current UX | Friction |
|---|---|---|
| AgentActivityPanel | Shows recent blocks by known `agent::` authors | Passive, manually refreshed, fixed agent list |
| Analysis API client | Calls `/analysis/mirror`, `/analysis/connections`, `/analysis/gardener` | REST routes are not mounted |
| MCP tools | Search/query/retrieval/temporal/graph/template/system | Strong backend portfolio, weak UI surfacing |

Recommendation:

| Priority | Proposal | Why |
|---|---|---|
| P0 | Align UI with mounted routes or mount analysis routes | Dead API surface creates broken cognitive features |
| P1 | Turn agent runs into first-class graph objects | Auditability, provenance, rollback, search |
| P1 | Add “Ask agent about this context” from block/page/query selection | Agent flow should start from existing context, not blank chat |
| P2 | Add agent permission scopes and action review | MCP-first architecture needs trust mechanics |

Lateral idea:

| Idea | Description |
|---|---|
| `Agent Workbench` | A panel that shows context pack, proposed actions, diff, confidence, and rollback for every agent operation. |

## SUNNY For Quilt

SUNNY's direct lesson is not “use a solver”. The reusable product principle is:

> Do not force the user to pick the perfect tool. Extract features from the problem, compare with similar cases, and schedule the best tools.

### Terminology Mapping

| SUNNY concept | Quilt equivalent |
|---|---|
| CSP instance | Current user context: block/page/query/navigation/task |
| Feature vector | Numeric/typed summary of that context |
| Portfolio solvers | Views, commands, templates, MCP tools, agents, query executors |
| k-nearest neighbors | Similar past pages/blocks/workflows/queries |
| Schedule | Ordered plan: show view, run query, suggest template, call agent, fallback |
| Backup solver | Safe default workflow: daily capture, FTS search, plain list view |
| Pre-solving | Cheap probes: classify context, inspect schema, quick search, count links |
| Solving | Execute selected strategy with time/effort budget |

### Feature Vectors Quilt Can Extract

Start simple. No machine learning required.

| Feature group | Examples |
|---|---|
| Content shape | text length, has TODO marker, has date, has `[[page]]`, has `((block))`, has `#tag`, has `key:: value` |
| Graph shape | backlinks count, outgoing refs count, unlinked refs count, child count, depth, page age |
| Schema shape | property keys, missing required fields, cardinality of selected property, template name |
| Usage context | current route, last 5 actions, mobile/desktop, time of day, recently used views |
| Query shape | result count, available sort fields, date fields, categorical fields, graph density |
| Agent shape | selected context size, confidence score, changed blocks count, author/source |

### Portfolio Actions

| Portfolio member | When it wins |
|---|---|
| Daily capture | Ambiguous input, mobile use, very short text |
| Page creation | Named concept, repeated mentions, many backlinks |
| Task conversion | TODO marker, date, priority, status-like text |
| Template application | Block/page resembles a known type |
| Table view | Many rows, many properties, low graph density |
| Kanban view | Categorical property with few values, e.g. status/priority |
| Graph lens | High link density or relationship exploration intent |
| Search | Short navigational query |
| Query builder | Structured filtering intent |
| Agent summarizer | Long page, many children, review intent |
| Agent organizer | Many untyped blocks, duplicate tags/properties |
| Agent researcher | Blocks contain questions, citations, sources, unknowns |

### Schedule Examples

Example: user types `show open tasks by project` in `Cmd+K`.

| Step | Action |
|---|---|
| Pre-solve | Detect task/status/project terms; inspect property keys; run cheap query count |
| Schedule 1 | Offer saved query preview as table |
| Schedule 2 | Offer Kanban grouped by project/status if cardinality is low |
| Schedule 3 | Offer “Create saved view” |
| Backup | Run FTS search for the literal text |

Example: user opens a page with many blocks and weak structure.

| Step | Action |
|---|---|
| Pre-solve | Count untyped blocks, repeated `key::`, repeated tags, backlinks |
| Schedule 1 | Suggest likely type/template |
| Schedule 2 | Show missing properties |
| Schedule 3 | Offer agent-assisted cleanup with diff |
| Backup | Leave normal outliner untouched |

Example: user clicks graph on a dense project page.

| Step | Action |
|---|---|
| Pre-solve | Local refs/backlinks within depth 2; classify node types |
| Schedule 1 | Open local graph lens, not global graph |
| Schedule 2 | Highlight decisions/tasks/evidence separately |
| Backup | Open normal graph route |

### Incremental Architecture

Do not overbuild this as ML first. Build the smallest useful selector.

| Phase | Scope | Implementation shape |
|---|---|---|
| 1 | Rule-based strategy selector | Pure TS/Rust function: `ContextFeatures -> RankedAction[]` |
| 2 | Telemetry memory | Store anonymized action outcomes: accepted, ignored, completed, undone |
| 3 | k-NN similarity | Compare feature vectors against successful prior workflows |
| 4 | Scheduled execution | Run cheap probes first, then suggest/run selected actions with budgets |
| 5 | Agent/MCP portfolio | Expose saved queries/views/actions as MCP tools and select among them |

### Data Model Sketch

```text
InteractionCase
  id
  timestamp
  context_route
  feature_vector
  candidate_actions
  selected_action
  outcome
  elapsed_ms
  reverted
```

```text
StrategyAction
  id
  label
  kind: view | command | query | template | agent | navigation
  cost: cheap | medium | expensive
  risk: safe | modifies_content | destructive
  required_capabilities
```

### Guardrails

| Risk | Guardrail |
|---|---|
| Annoying suggestions | Only show high-confidence suggestions inline; keep the rest in command center |
| Wrong automation | Never auto-modify content without preview/diff/undo |
| Privacy | Store local interaction history; no external calls needed |
| Complexity creep | Start with rules, not model training |
| Non-determinism | Deterministic ranking first; k-NN only reranks after enough evidence |

## Design System Direction

Borrowing from the provided UI/UX Pro Max checklist, Quilt should optimize for a professional, productivity-focused knowledge-work interface:

| Area | Recommendation |
|---|---|
| Accessibility | Keep keyboard-first navigation, visible focus, labels on icon-only buttons, `aria-live` for toasts/errors |
| Touch | Mobile primary flows should be capture, search, daily journal, review; avoid tiny drag-only interactions |
| Performance | Virtualize long lists, cache page/tag lookup, avoid N calls from each block row |
| Navigation | One command center; persistent core navigation; no hidden expert routes unless discoverable |
| Forms | Replace `window.prompt` with modal/sheet forms with validation, loading, error recovery |
| Motion | Use small state transitions for panels, command results, view morphing; respect reduced motion |
| Data | Tables need sorting, saved filters, accessible summaries; charts/graph need text alternatives |
| Style | Preserve existing design tokens, but reduce mixed Spanish/English copy and ad-hoc inline visual decisions |

## Prioritized Roadmap

### P0: Trust And Broken Workflow Fixes

| Item | Outcome |
|---|---|
| Redirect `/` to today's journal or a useful dashboard | No empty app entry |
| Remove or fix broken QueryPage | Expert features stop lying |
| Add property-key/schema endpoint or stop calling `getBlockProperties('')` | Table/Kanban can work reliably |
| Align SSE/analysis/schema-pack API client with mounted server routes | No phantom capabilities |
| Persist destructive block actions correctly | No data resurrection |
| Replace core `window.prompt` flows with real modal forms | Better validation and accessibility |

### P1: Unified Workflows

| Item | Outcome |
|---|---|
| Universal command center | Search, commands, templates, views, agents in one place |
| Saved View model | Query/table/kanban/list/graph/cards unify around one object |
| Quick capture + triage | Daily-first workflow becomes the main habit loop |
| Template contracts with preview | Templates become safe structure generators |
| Local graph lens | Graph becomes useful instead of decorative |
| Agent context packs and audit trail | Agents become trustworthy collaborators |

### P2: SUNNY-Inspired Intelligence

| Item | Outcome |
|---|---|
| Context feature extraction | Quilt understands the shape of the current task |
| Strategy selector | Quilt ranks views/actions/templates/agents |
| Outcome memory | Quilt learns from accepted/ignored/undone suggestions |
| k-NN reranking | Similar contexts recommend similar successful workflows |
| Query-as-tool for MCP | Saved views become agent-usable tools |

## Open Product Decisions

| Decision | Recommendation |
|---|---|
| Is Quilt primarily Logseq-like or object/database-like? | Keep outliner/daily as default, add object/view power progressively |
| Should cognitive features be visible in UI if ADR says no internal AI? | Yes, but as MCP/agent orchestration, not internal LLM magic |
| Should raw DSL be exposed? | Only after it is real and documented; default should be builder/search-first |
| Should templates auto-apply? | Only with preview/undo, except for safe empty-page defaults |
| Should SUNNY selector auto-run actions? | Initially no. Suggest first, then allow user-approved automation. |

## Key Product Principle

Quilt should not ask the user to understand its internal feature map before getting value.

The product should behave like this:

```text
User intent -> context features -> ranked strategy -> safe preview -> action -> learned outcome
```

That is the real reusable idea from SUNNY: not one perfect workflow, but a portfolio of workflows selected intelligently.

## Resolved Design Forks (Auto-Grill Session 2026-06-07)

After 14 grill cycles (5 accepted, 9 rejected, BLOCKED at 42% coverage), three forks were resolved by architect decision:

### AgentRun: Block Role, Not Entity

**Decision**: AgentRun is a block with `type:: agent-run` and properties — NOT a separate domain entity.

| Property | Type | Purpose |
|----------|------|---------|
| `agent::` | string | Which agent (e.g. claude, gemini) |
| `model::` | string | Model identifier |
| `run-status::` | enum | Queued | Running | Completed | Failed | Cancelled |
| `started-at::` | timestamp | Run start |
| `completed-at::` | timestamp | Run end |
| `context-page::` | page-ref | Page context |
| `summary::` | text | Result summary |
| `blocks-modified::` | block-ref[] | UUIDs of modified blocks |

**Rationale**: Quilt's role system IS its type system. Lifecycle modeled via `run-status::` property (same pattern as `status:: todo/done`). Queryable via DSL. Zero migrations. Consistent with ADR-0003.

### SavedView: Block Role Composing Query Reference

**Decision**: SavedView is a block with `type:: view` (existing CONTEXT.md role) composing a reference to a query block via `data-source::`.

| Property | Type | Purpose |
|----------|------|---------|
| `type::` | role | view |
| `view-type::` | enum | table | kanban | calendar | list | graph | cards | timeline |
| `data-source::` | block-ref | UUID of the query block |
| `view-name::` | string | Human-readable name |
| `view-icon::` | string | Lucide icon name |
| `view-pinned::` | boolean | Pin to sidebar/command center |
| `group-by::` | property-key | Grouping property |
| `sort::` | json | Sort configuration |

**Rationale**: Composition over inheritance. Multiple views can reference the same query. Follows existing `((block-ref))` pattern. Aligns with Logseq/Tana outliner-native approach.

### StrategySelector: Two Traits in quilt-core (WASM)

**Decision**: StrategySelector + StrategyScorer traits in quilt-core. Phase 1: deterministic rules, no persistence. Exposed via MCP.

| Component | Crate | Role |
|-----------|-------|------|
| StrategySelector trait | quilt-core | `fn select(features, scorer, portfolio) -> Vec<RankedAction>` |
| StrategyScorer trait | quilt-core | `fn score(action, features) -> f32` |
| ContextFeatures struct | quilt-core | ContentShape + GraphShape + SchemaShape + UsageContext |
| RuleBasedSelector | quilt-application | Phase 1 implementation |
| FeatureExtractor | quilt-application | Reads context from repositories |
| useStrategySuggestions hook | quilt-ui | React hook calling WASM |
| quilt_strategy_select tool | quilt-mcp | MCP tool wrapper |

**Phase 1 scope**: Content shape, graph shape, usage context. 6-8 portfolio actions. Top 3 shown as hints. No auto-apply. No telemetry.

**Rationale**: WASM for sub-100ms browser latency. Trait separation for independent testing. Phase 3 k-NN implements same traits. MCP-first: agents call same selector.

## Grill Session Artifacts

- Final report: `docs/grill/2026-06-07-ux-workflow-portfolio-analysis.report.md`
- Ledger: `docs/grill/.state/2026-06-07-ux-workflow-portfolio-analysis.ledger.md`
- Summary: `docs/grill/.state/2026-06-07-ux-workflow-portfolio-analysis.summary.md`
- Cycle evidence: `docs/grill/.state/cycles/Q*`
