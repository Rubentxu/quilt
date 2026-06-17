# Quilt — Action Roadmap

> Generated: 2026-06-07
> Last updated: 2026-06-17 (Phase 10 SDDK execution completed — bootstrap, validation, global state, selector, tests)
> Sources: auto-grill 30+ cycles, architecture review (7 candidates), SDDK kernel orchestrator (init → explore → propose → spec → design → tasks → apply → verify → archive)

## Changelog
- 2026-06-14: Phase 6 (Architecture Deepening) completed
- 2026-06-15: Phase 7 (ADR-0025: Property-First Architecture) completed — 5 slices, 12,800 LOC, 3,000+ tests
- 2026-06-16: Phase 8 (Post-ADR-0025 cleanup) completed:
  - ADR-0026: Deprecation cleanup (33 deprecation warnings → 0)
  - ADR-0027: Typed PropertyValue (Url + NaiveDate variants)
  - ADR-0028: WASM client-side projection
  - ADR-0029: Pre-existing test fixes (7 tests)
  - OpenSpec cleanup: 8 done items archived to `archive/done-2026-06-16/`; INDEX.md created
  - Roadmap-gaps P0 docs: 3 issues (schema unification, sync strategy, task markers)
  - Phase 9 CG-1 Morning Briefing end-to-end: backend endpoint + UI component + 11 tests
  - Fixed 2 MorningBriefing test bugs (getAllByText + getByRole)
  - OpenSpec cleanup: 6 done items archived, 16 active grouped by category → INDEX.md
- 2026-06-17: ADR-0030 proposed — Graph Space, canonical storage, and journal-first lifecycle
- 2026-06-17: ADR-0030 ratified to `accepted` after SDDK verification (PASS WITH WARNINGS)
- 2026-06-17: **Phase 10 applied**: bootstrap unification (Slice A ✅), graph validation (Slice B ✅), global app state + REST endpoints (Slice C ✅), graph selector + startup routing (Slice D ✅)
- 2026-06-17: Phase 10 tests added: GraphSelectorPage (21 tests), HomePage conditional routing (7 tests), api-client surface update, E2E graph-selector spec
- 2026-06-17: SDDK archive completed — change `graph-space-migration-phases-1-4` synced to openspec/ + engram
- 2026-06-17: Graph Space migration plan documented in `docs/graph-space-migration-plan.md`

**Leyenda**: ✅ completado | 🚧 en progreso | 🔲 pendiente | `commit` = commit SHA

---

## 0. Estado Actual

### ✅ Funciona (post Phase 0-5)

- Backend: Axum + SQLite + FTS5 + MCP tools + property CRUD + `blockType` persistence
- Frontend: React 19 + TipTap + sidebar + block editor + search modal + templates
- WASM: quilt-core compila a wasm32-unknown-unknown
- Dev: `just dev` — hot reload completo
- Home redirect `/` → journal de hoy ✅
- Graph dark mode con `data-theme` ✅
- CutBlock + UndoManager (Cmd+Z) ✅
- CommandRegistry + Cmd+Shift+K palette ✅
- Property keys desde API real ✅

### Bloqueantes resueltos

- `blockType` persiste en backend ✅ (Phase 0 #6)
- `cognitive.rs` eliminado ✅ (Phase 1 #10)
- API client alineado con rutas montadas ✅ (Phase 0 #4)
- QueryPage roto eliminado ✅ (Phase 0 #2)

### Slash commands `/` — 32 registrados (Phase 1 + Phase 2)

Infraestructura: `SlashActionRegistry` en `slashRegistry.tsx`, lazy-loaded desde `BlockRow`.

| Categoría | Comandos | Qué hace |
|-----------|----------|----------|
| Status | `/todo` `/doing` `/done` `/now` `/later` `/cancelled` | `api.updateBlock({ marker })` |
| Priority | `/priority A` `/B` `/C` | `api.updateBlock({ priority })` |
| Dates | `/today` `/tomorrow` | Inserta fecha como texto |
| Date props | `/deadline` `/scheduled` | Inserta `prop:: ` syntax |
| References | `/page reference` `/block embed` | Inserta `[[` / `((` |
| Templates | `/new from template` | Wizard: pick → create → navigate |
| Comments | `/add comment` | `onAddComment` callback |
| Block types | `/text` `/h1`-`/h3` `/bullet` `/numbered` `/todo` `/quote` `/code` `/divider` `/image` | `api.updateBlock({ blockType })` ✅ persiste |
| Roles (Phase 2) | `/task` `/query` `/card` | Establece `type:: task`, `type:: query` + `dsl::`, `card-shape::` ✅ |

---

## 1. ADR Pipeline

### Canónicos vigentes (0001-0030)

| ADR | Decisión | Estado |
|-----|----------|--------|
| 0001 | No IA interna, MCP-first | ✅ |
| 0002 | UI Logseq-like — paneles, no vistas separadas | ✅ |
| 0003 | Colaboración humano-agente via properties, sin estados fijos | ✅ |
| 0004 | DSL compartido como superconjunto + MCP | ✅ |
| 0005 | No Tauri/Leptos — React 19 CSR | ✅ |
| 0006 | Outliner como fuente de verdad | ✅ |
| 0007 | TipTap como editor + template-driven block cards | ✅ |
| 0008 | Refs bidireccionales — RefIndex en dominio | ✅ |
| 0009 | Formato inline Logseq-compatible | ✅ |
| 0010 | Testing strategy | ✅ |
| 0011 | Cognitive* family (3 paneles) + namespace `cognitivo::` | ✅ `28afeb2` |
| 0012 | CommandRegistry + Cmd+Shift+K palette | ✅ `69c254b` |
| 0013 | GET /properties/keys con cursor pagination | ✅ `c435c6f` |
| 0014 | StrategySelector + StrategyScorer traits + `RelevanceScorer` impl | ✅ `35af73a` |
| 0015 | AgentRun inline rendering en BlockRow | ✅ `06e28ad` |
| 0016 | SavedViewBlock + type:: view + data-source:: | ✅ `ses_15c8a3d` |
| 0017 | PanelVisibilityContext + presets + LayoutMenu | ✅ `06e28ad` |
| 0018 | GET /blocks/authors (dinámico) + GET /pages/search server-side | ✅ |
| 0019 | Property Intelligence batch-first (PI-1..PI-3) | ✅ `c35b178` |
| 0020 | Property editing surface fixed header | ✅ |
| 0021 | Property indexing virtual columns | ✅ |
| 0022 | Template-driven block cards | ✅ |
| 0023 | Block renderer registry | ✅ |
| 0024 | Property view renderers (Gallery, List, Table) | ✅ |
| 0025 | Property-first architecture | ✅ |
| 0026 | Deprecation cleanup | ✅ |
| 0027 | Typed PropertyValue | ✅ |
| 0028 | WASM client-side projection | ✅ |
| 0029 | Pre-existing test fixes | ✅ |
| 0030 | Graph Space + canonical storage + journal-first lifecycle | ✅ accepted |

---

## 2. CONTEXT.md

Actualizado con nuevos términos del roadmap:

**Agregado**: AgentRun ✅, SavedView ✅, DashboardLayout ✅, CommandRegistry ✅, ViewContainer, Cognitive* family, StrategySelector, Card Shape, Card Renderer, Propuesta, Annotation, Link (rol), Acción, Query embebida, Grafo pesado

**Modificado**: Rol (agregar `agent-run` ✅, `insight`), View (agregar `data-source::` ✅), Query embebida (referencia a view)

**Eliminado**: Serendipity Feed, Agent Workbench, ConnectionType::Semantic/Content

> ✅ Integrado en CONTEXT.md global

---

## 3. Fases de Implementación

### Phase 0: P0 Fixes ✅ COMPLETADO

| # | Qué | Status |
|---|-----|--------|
| 1 | Home redirect `/` → journal de hoy | ✅ |
| 2 | Eliminar QueryPage roto | ✅ |
| 3 | Fix graph dark mode (`data-theme`) | ✅ |
| 4 | Alinear API client con rutas montadas | ✅ |
| 5 | CutBlock + UndoManager (Cmd+Z) | ✅ |
| 6 | Backend: agregar `blockType` a `BlockDto` + `UpdateBlockRequest` | ✅ |

### Phase 1: Fundamentos ✅ COMPLETADO

| # | Qué | Status |
|---|-----|--------|
| 7 | `GET /api/v1/properties/keys` | ✅ |
| 8 | Frontend usa endpoint real | ✅ |
| 9 | CommandRegistry + Cmd+Shift+K | ✅ |
| 10 | Renombrar paneles → Cognitive* family | ✅ |
| 11 | Remover tree_rag dead code | ✅ |

### Phase 2: UX Block-Level ✅ COMPLETADO

| # | Qué | Status |
|---|-----|--------|
| 12 | Block Zoom (`?zoom=$blockId`) | ✅ |
| 13 | Inline+Panel Properties (template) | ✅ |
| 14 | Quick Capture (CommandRegistry) | ✅ |
| 15 | Natural Language Dates V1 | ✅ |
| 16 | Commandable Transforms (`/task`, `/query`, `/card`) | ✅ |
| 17 | DashboardLayout + PanelVisibility | ✅ |
| 18 | Cognitive* panel implementations | ✅ |
| 19 | AgentRun block role | ✅ |
| 20 | SavedView block role | ✅ |

### Phase 3: Infra + Avanzado ✅ COMPLETADO

| # | Qué | Status |
|---|-----|--------|
| 21 | Session cache V1 | ✅ |
| 22 | Saved/Recent searches | ✅ |
| 23 | Graph Lens V1 (subgraph endpoint) | ✅ |
| 24 | StrategySelector traits (determinístico) | ✅ |
| 25 | "Save as View" desde search | ✅ |
| 26 | StrategySelector WASM + hook | ✅ |
| 27 | Graph Lens V2 (lens buttons) | ✅ |

### Phase 4: Re-grill Remedies ✅ COMPLETADO

| # | Qué | Status |
|---|-----|--------|
| 28 | Editable Backlinks | ✅ |
| 29 | Unlinked Ref Queue | ✅ |
| 30 | Template Contracts | ✅ |
| 31 | Template Doctor | ✅ |

### Phase 5: Property Intelligence ✅ COMPLETADO (2026-06-11)

| ID | Qué | Commit |
|----|-----|--------|
| PI-1 | DSL Aggregate/Stats/GroupBy/SortBy + MCP wiring | `12da876` |
| PI-2 | PropertyStatus lifecycle + key validation + batch repo methods | `c35b178` |
| PI-3 | SqlitePropertyRepository + PropertyService + REST/MCP handlers | `c6993b4` |
| PI-4 | Property discovery: fuzzy suggest endpoint + MCP tool | `f9c1b9c` |
| PI-5 | Analytics: co-occurrence PMI + usage trends | `be18a7d` |
| PI-6 | Lifecycle management: deprecate, merge, alias | `9ce4ad2` |
| PI-7 | Schema templates with auto-detect from co-occurrence | `1386a85` |
| PI-8 | Semantic property graph with directed relations | `cba94d4` |
| — | PropertyStrip: compact property card below block content | `cd53117` |

### Phase 6: Architecture Deepening ✅ COMPLETADO (2026-06-14)

| ID | Qué | Commit |
|----|-----|--------|
| AD-1 | Extract business logic from handlers → BlockUseCases | `5012e7b` |
| AD-2 | Clean 7 dead repository trait files (−424 lines) | `3e614be` |
| AD-3 | Unify REST + MCP serialization → canonical BlockDto | `f6aa983` |
| AD-4 | Standardize handler patterns: Extension<Arc<dyn T>> | `5e024b0` |
| AD-5 | Consolidate 9 Arc<dyn Trait> → RepositoryBundle | `4ee1a44` |
| AD-6 | Delete dead helpers.rs (not in module tree) | `aa98546` |
| AD-7 | Query objects — rejected (not worth the indirection) | — |
| — | RefServiceTrait: extract trait, encapsulate RwLock | `5012e7b` |
| — | BlockUseCasesImpl: non-generic, injected RefServiceTrait | `5012e7b` |
| — | MigrationEngine: collapse generics to Arc<dyn Trait> | `a6c56e7` |
| — | Remove pool from AppState (all handlers migrated) | `38a2d8f` |
| — | Path traversal hardening (canonicalize + symlinks + file limit) | `38a2d8f` |
| — | Warning cleanup: 0 warnings across quilt-server | varios |

### Judgment Day Fixes ✅ (2026-06-09)

| # | Finding | Severity | Fix | Commit |
|---|---------|----------|-----|--------|
| C1 | `isTaskBlock` no matcheaba `type:: task` | CRITICAL | Extendido | local |
| S1-03 | `cognitive.rs` orphaned | CRITICAL | Deleted | local |
| S1-04 | `blockMatchesFilter` regex → false positives | CRITICAL | Properties lookup | SDD |
| S2-01 | PANEL_LABELS dual source | WARNING | Canonical | local |
| S2-02 | AgentActivityFeed hardcoded agents | WARNING | Dinámico | local |
| S2-03 | SearchModal O(n) carga páginas | WARNING | Server-side | local |
| S2-04 | StrategySelector sin impl | WARNING | RelevanceScorer | local |
| S2-05 | graph.rs depth bounds | WARNING | Constants | local |
| S2-06 | template_doctor.rs 49K lines | WARNING | Split 4 módulos | local |

### Phase 7: Property-First Architecture (ADR-0025) ✅ COMPLETADO (2026-06-15)

**Gran cambio arquitectónico**: Properties son source of truth del Bloque (semántica, estructura, proyección, metadata). Elimina 23 referencias hardcodeadas a `block.marker`/`block.blockType` en `BlockRow.tsx`.

| Slice | Qué | Commit |
|-------|-----|--------|
| **#1** Property Configuration Domain Model | PropertyVisibility, PropertyMutability, DerivedSource, MergePolicy + 4 fields en PropertyDefinition + from_legacy_fields + assert_invariants + WASM mirror | `5e41eb1`..`ee35d72` |
| **#2** Input Canonicalization Pipeline | Canonicalizer trait + MarkdownCanonicalizer + 4 VOs + PropertyPatch::apply_to con 6 MergePolicies + proptests | `b63bde0`..`2a00b1e` |
| **#3** Slash Command Property Presets | PropertyPreset, PresetRegistry, StaticPresetRegistry::v1 con 9 V1 presets, ApplyPreset use case, cross-feature equivalence, **fix task-marker gap** | `44027c0`..`256aad8` |
| **#4** Projection Resolver | PropertyPredicate, ProjectionContract, ProjectionView, Projection trait, DefaultProjection, ProjectionLayerConflict, ProjectionRegistry, 6 V1 contracts, ProjectionResolver use case, conflict materialization, 1000-iter proptest | `620f385`..`ce2cd95` |
| **#5** UI Surface Refactor | 2 server endpoints (projection + presets), ProjectionRenderer, PresetMenu, SystemPropertyToggle, BlockPropertiesPanel refactored (visibility + mutability), BlockRow delegates to ProjectionRenderer, slashRegistry consumes PresetRegistry, edit mode shows all properties | `e0beb4a`..`ea67da7` |

**Cambios de raíz en main**:
- 23 referencias hardcodeadas a `block.marker`/`block.blockType` eliminadas
- Single source of truth para projection en el `ProjectionResolver` (Rust)
- Visualización Genérica de Texto como fallback obligatorio en conflictos
- Conflict materialization como system properties (`projection-conflict`, `projection-conflict-reason`, `projection-conflict-candidates`)
- Slash commands = Property Presets no destructivos
- Canonización de Entrada: paste/Markdown/slash/API/MCP → Block Content + Property Patches

**Tests**: 432 + 514 + 557 + 636 + 1289 (UI) + 25 (integration) + 1000-iter proptest = **~3500 tests passing**.

**OpenSpec artifacts** (archived in `openspec/changes/archive/adr-0025-merged/`):
- `property-configuration-domain-model/`
- `input-canonicalization-pipeline/`
- `slash-command-property-presets/`
- `projection-resolver-declarative-contracts/`
- `ui-surface-refactor-projection-aware/`

### Phase 8: Post-ADR-0025 Cleanup ✅ COMPLETADO (2026-06-16)

| ADR | Qué | Commit |
|-----|-----|--------|
| **0026** Deprecation Cleanup | 33 deprecation warnings → 0 (5 legacy fields removed: view_context, public, queryable, hidden, read_only) | `2358b27` |
| **0027** Typed PropertyValue | `Url(url::Url)` y `NaiveDate(chrono::NaiveDate)` variantes + slice #3 workaround removido | `abf4467` |
| **0028** WASM Client-Side Projection | ProjectionResolver portado a `quilt-core` WASM + 6 V1 contracts reimplementados como funciones puras + `useProjection` con WASM-first + HTTP-fallback + 18-row parity test + metrics | `2abfe38` |
| **0029** Pre-existing Test Fixes | 7 tests fixed: order_proptest + GraphViewPage (6) + JournalAggregator | `8437ebb` + `105983a` + `c78eb36` |

### Phase 10: Graph Space + Journal-First Lifecycle 🚧 EN PROGRESO (2026-06-17)

**Objetivo**: consolidar Quilt como aplicación de Graph Space local-first con `quilt.db` canónica dentro del graph, Journal de hoy como puerta de entrada, y panel derecho contextual como superficie operativa secundaria.

| ID | Qué | Estado |
|----|-----|--------|
| GS-1 | Unificar bootstrap de Graph en server, CLI y MCP (directorio → `.quilt/quilt.db`) | ✅ |
| GS-2 | Persistir estado global de app (`last_opened_graph`, recientes, layout global) fuera del Graph | ✅ |
| GS-3 | Validación explícita de Graph inválido (sin autoreparación silenciosa) | ✅ |
| GS-4 | Selector de Graph solo como fallback cuando no haya `last_opened_graph` válido | ✅ |
| GS-5 | Apertura siempre en Journal de hoy + creación bajo demanda del día actual | ✅ |
| GS-6 | Morning Briefing colapsable y visible por defecto solo para hoy vacío/recién creado | 🔲 |
| GS-7 | Metadata de Graph Space dentro del graph (nombre, icono, descripción, color, fecha) | 🔲 |
| GS-8 | Panel derecho contextual visible por defecto, colapsable y panel-first para properties | 🔲 |
| GS-9 | Ingesta/reindex manual de recursos compatibles en directorios existentes | 🔲 |
| GS-10 | Local Graph v1: 2D, contextual, profundidad 1/2/3, navegación bidireccional | 🔲 |

**Referencia**:
- ADR: `docs/adr/0030-graph-space-journal-first-lifecycle.md` (ratificado ✅)
- Plan: `docs/graph-space-migration-plan.md`
- SDDK: `openspec/changes/graph-space-migration-phases-1-4/` (archivado)
- Progreso: 5/10 items ✅ — slices A–D implementados y verificados con 28 tests nuevos

---

## 4. Dependencias (estado actual)

```
Phase 0 ✅ COMPLETO (P0 Fixes)
Phase 1 ✅ COMPLETO (Fundamentos)
Phase 2 ✅ COMPLETO (UX Block-Level)
Phase 3 ✅ COMPLETO (Infra + Avanzado)
Phase 4 ✅ COMPLETO (Re-grill Remedies)
Phase 5 ✅ COMPLETO (Property Intelligence PI-1..PI-8)
Phase 6 ✅ COMPLETO (Architecture Deepening, 7 candidates)
Phase 7 ✅ COMPLETO (ADR-0025: Property-First Architecture, 5 slices)
Phase 8 ✅ COMPLETO (Post-ADR-0025 cleanup: 0026/0027/0028/0029)
Phase 9 ✅ COMPLETO (UI Cognitive: CG-1..CG-7)
Phase 10 🚧 EN PROGRESO (Graph Space: 5/10 — bootstrap + lifecycle foundations)
```

---

## 5. Estado Final — TODAS LAS FASES COMPLETADAS

**Phase 0-9 ✅ — 31 items originales + 8 PI items + 7 AD items + 5 ADR-0025 slices + 4 post-ADR-0025 ADRs + 7 CG items completados**
**Phase 10 🚧 — 5/10 items de Graph Space foundations implementados**

### Pendiente técnico (no-bloqueantes)

- 🔲 Reconciliar documentación con schema real (roadmap-gaps P0)
- 🔲 Formalizar Query DSL como contrato documentado
- 🔲 User manual (HTML) — deploy o hosting
- 🔲 Documentar estrategia real de sync (LWW vs CRDT visión) — `roadmap-gaps/ ISSUE-002`
- 🔲 Normalizar task markers casing — `roadmap-gaps/ ISSUE-003`
- 🔲 Pre-existente: doctests en `quilt-domain` (presets, graph_builder) y errores `quilt-bin` (chrono/migrate-comments) — ajenos a ADRs

### OpenSpec work items activos

> 16 items activos (2026-06-16). Ver categorization completa en [`openspec/changes/INDEX.md`](../openspec/changes/INDEX.md).

| Item | Tipo | Estado |
|------|------|--------|
| `petgraph-graph-engine` | Foundation | Pendiente (Graph V2) |
| `query-refactor-v1` | Query | Pendiente |
| `intent-search-v3a` | Search | Pendiente (V3+ deferido) |
| `retrieval-graph-v1` | Search | Pendiente |
| `journal-editing-and-config` | UX | ⚠️ STALE post-React migration |
| `outliner-keyboard` | UX | ⚠️ STALE post-React migration |
| `quilt-fase2-ux-dead-buttons` | UX | Pendiente |
| `quilt-fase2-ux-empty-states` | UX | Pendiente |
| `quilt-architecture-review` | Arch Review | Pendiente (umbrella) |
| `architecture-review-2026-06-11` | Arch Review | Reporte (`reports/`) |
| `quilt-architecture-review-c1-keyboard` | Arch Review | Pendiente |
| `quilt-architecture-review-c3-serialize` | Arch Review | Pendiente |
| `quilt-architecture-review-c4-slash-registry` | Arch Review | Pendiente |
| `quilt-architecture-review-c5-template-hook` | Arch Review | Pendiente |
| `quilt-fase3-backlog-e2e-template-flow` | E2E | Pendiente |
| `quilt-fase3-backlog-small-fixes` | E2E | Pendiente |
| `quilt-fase4-cross-device-tour` | Onboarding | Pendiente |
| `quilt-fase4-onboarding-advanced` | Onboarding | Pendiente |

**Archivados (2026-06-16)**: `domain-properties-v1`, `slash-command-functional-behavior`, `dsl-aggregates`, `dsl-analyze`, `evidence-contract-v1`, `frontend-templates-v1` → `openspec/changes/archive/done-2026-06-16/`
**Archivados (2026-06-17)**: `graph-space-migration-phases-1-4` → `openspec/changes/graph-space-migration-phases-1-4/` (SDDK completo: init → explore → propose → spec → design → tasks → apply → verify → archive)

### Phase 9: UI Cognitive ✅ COMPLETO

| ID | Qué | Backend | Estado |
|----|-----|---------|--------|
| CG-1 | Morning Briefing end-to-end | ✅ Existe | ✅ DONE (`af8191f`, `c9bf6b9`) |
| CG-2 | Cognitive Dashboard / Graph View | ✅ Existe | ✅ DONE (`9c287cc`) |
| CG-3 | Serendipity UI | ✅ Existe | ✅ DONE (`08fee0d`) |
| CG-4 | Query UI avanzada | ✅ Existe | ✅ DONE (`962dcd0`) — see CG-4 followup below |
| CG-5 | Agent Room multi-agente | — | ✅ DONE V1 (`336223b`) — see CG-5 V2 followups |
| CG-6 | Focus mode con AI panel | — | ✅ DONE V1 (`6cd32b1`) — see CG-6 V2 followups |
| CG-7 | Decay monitor + weekly review | ✅ Existe | ✅ DONE (`a6a10c3`) |

### CG-4 followup: Parser position tracking

`quilt-query/src/parser.rs` currently emits `ParseError::Syntax(String)` and `ParseError::Invalid(String)` without line/column. The CG-4 error UI shows the message + a "Show in docs" link, but lacks a clickable caret for fast navigation. Two options:
- **A**: redesign the parser to track position (separate workitem, weeks of work)
- **B**: accept the current UX (message + docs link) — already shipped

Decision: keep B; revisit A in a future workitem if user feedback requests it.

### CG-5 V2 followups
- WebSocket/SSE migration (replace polling)
- 2nd agent type (e.g., "Cross-Reference Finder" using Serendipity engine)
- Historical agent persistence

### CG-6 V2 followups
- Focus mode persistence (remember per-page)
- Visual toggle button
- Agent status polling in the AI panel

### Diferido (V3+)

- Block Shape Detector
- Intent Search completo (NL→DSL)
- SUNNY Phase 2-5 (telemetry, k-NN, auto-execute)
- E2EE
- File watching end-to-end
- WASM como producto completo (más allá de compilación)
- Typed PropertyValue V2 (full AST propagation)
- Dynamic PropertyPresets/ProjectionRegistry loaders (plugins)
