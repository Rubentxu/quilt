# Quilt — Action Roadmap

> Generated: 2026-06-07
> Last updated: 2026-06-14 (post Architecture Deepening + Pool Removal + Property Intelligence PI-1..PI-8)
> Sources: auto-grill 30+ cycles, architecture review (7 candidates), git log (commits be18a7d..aa98546)

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

### Canónicos vigentes (0001-0024)

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

### ADR Drafts (pendientes de promover)

| Draft | Decisión | Estado |
|-------|----------|--------|
| DRAFT-migration-engine-collapse-generics | Collapse MigrationEngine generics to Arc<dyn Trait> | 🔲 Promover a ADR-0025 |
| DRAFT-migration-path-traversal-hardening | Path traversal hardening for migration import | 🔲 Promover a ADR-0026 |
| DRAFT-refindex-internal-synchronization | RefIndex sync via RefServiceTrait with internal RwLock | 🔲 Promover a ADR-0027 |
| DRAFT-appstate-extension-pattern | AppState Extension pattern for FromRef + Extension injection | 🔲 Promover a ADR-0028 |
| DRAFT-test-organization-pattern | Inline #[cfg(test)] over separate /tests directories | 🔲 Promover a ADR-0029 |

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

---

## 4. Dependencias (estado actual)

```
Phase 0 ✅ COMPLETO
Phase 1 ✅ COMPLETO
Phase 2 ✅ COMPLETO
Phase 3 ✅ COMPLETO
Phase 4 ✅ COMPLETO
Phase 5 ✅ COMPLETO (Property Intelligence PI-1..PI-8)
Phase 6 ✅ COMPLETO (Architecture Deepening, 7 candidates)
```

---

## 5. Estado Final — TODAS LAS FASES COMPLETADAS

**Phase 0-6 ✅ — 31 items originales + 8 PI items + 7 AD items completados**

### Pendiente técnico (no-bloqueantes)

- 🔲 Promover 5 ADR drafts (0025-0029)
- 🔲 User manual (HTML) — deploy o hosting
- 🔲 Reconciliar documentación con schema real (roadmap-gaps P0)
- 🔲 Formalizar Query DSL como contrato documentado

### Phase 7: UI Cognitive (propuesta — próximo)

| ID | Qué | Backend | Falta |
|----|-----|---------|-------|
| CG-1 | Morning Briefing end-to-end | ✅ Existe | UI render + estados |
| CG-2 | Cognitive Dashboard / Graph View | ✅ Existe | Visualización clusters |
| CG-3 | Serendipity UI | ✅ Existe | Feed sugerencias |
| CG-4 | Query UI avanzada | ✅ Existe | Builder UX + feedback |
| CG-5 | Agent Room multi-agente | — | Diseño + implementación |
| CG-6 | Focus mode con AI panel | — | Diseño + implementación |
| CG-7 | Decay monitor + weekly review | ✅ Existe | UI workflow |

### Diferido (V3+)

- Block Shape Detector
- Intent Search completo (NL→DSL)
- SUNNY Phase 2-5 (telemetry, k-NN, auto-execute)
- E2EE
- File watching end-to-end
- WASM como producto completo (más allá de compilación)
