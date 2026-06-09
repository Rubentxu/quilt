# Quilt — Action Roadmap

> Generated: 2026-06-07
> Last updated: 2026-06-09 (post Phase 1-2 sprint)
> Sources: auto-grill 30 cycles, ux-workflow-portfolio-analysis.md, 7 ADR drafts, CONTEXT.md patch

**Leyenda**: ✅ completado | 🚧 en progreso | 🔲 pendiente | `commit` = commit SHA

---

## 0. Estado Actual

### ✅ Funciona (post Phase 0-1)
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

### Canónicos vigentes (0001-0010)

| ADR | Decisión |
|-----|----------|
| 0001 | No IA interna, MCP-first |
| 0002 | UI Logseq-like — paneles, no vistas separadas |
| 0003 | Colaboración humano-agente via properties, sin estados fijos |
| 0004 | DSL compartido como superconjunto + MCP |
| 0005 | No Tauri/Leptos — React 19 CSR |
| 0006 | Outliner como fuente de verdad |
| 0007 | TipTap como editor + template-driven block cards |
| 0008 | Refs bidireccionales — RefIndex en dominio |
| 0009 | Formato inline Logseq-compatible |
| 0010 | Testing strategy |

### Implementados (de drafts)

| ADR | Draft | Implementación | Commit |
|-----|-------|---------------|--------|
| 0011 | DRAFT-dashboard-layout-no-work-modes.md | PanelVisibilityContext + presets + LayoutMenu | 06e28ad |
| 0012 | DRAFT-command-registry-mcp-dispatch.md | CommandRegistry + Cmd+Shift+K palette | 69c254b |
| 0015 | DRAFT-agent-run-block-role.md | AgentRun inline rendering en BlockRow | 06e28ad |
| 0016 | DRAFT-saved-view-block-role.md | SavedViewBlock + type:: view + data-source:: | ses_15c8a3d |

### Candidatos a promover

| Target | Draft | Decisión | Confianza |
|--------|-------|----------|-----------|
| 0013 | DRAFT-cognitive-panel-family-namespace.md | Cognitive* bajo namespace `cognitivo::` | Alta |
| 0014 | DRAFT-property-schema-endpoint.md | GET /properties/keys con cursor pagination | Media |
| 0017 | DRAFT-strategy-selector-trait-contract.md | StrategySelector + StrategyScorer traits en quilt-core (WASM) | Alta |

---

## 2. CONTEXT.md

Aplicar patch desde `docs/grill/.state/CONTEXT.patch.md`:

**Agregar**: AgentRun ✅, SavedView ✅, DashboardLayout ✅, CommandRegistry ✅, ViewContainer, Cognitive* family, StrategySelector

**Modificar**: Rol (agregar `agent-run` ✅, `insight`), View (agregar `data-source::` ✅), Query embebida (referencia a view)

**Eliminar**: Serendipity Feed, Agent Workbench, ConnectionType::Semantic/Content

> ⚠️ Patch NO aplicado aún. Pending de aplicar.

---

## 3. Fases de Implementación

### Phase 0: P0 Fixes ✅ COMPLETADO

| # | Qué | Status |
|---|-----|--------|
| 1 | Home redirect `/` → journal de hoy | ✅ c435c6f |
| 2 | Eliminar QueryPage roto | ✅ c435c6f |
| 3 | Fix graph dark mode (`data-theme`) | ✅ c435c6f |
| 4 | Alinear API client con rutas montadas | ✅ c435c6f |
| 5 | CutBlock + UndoManager (Cmd+Z) | ✅ c435c6f |
| 6 | Backend: agregar `blockType` a `BlockDto` + `UpdateBlockRequest` | ✅ c435c6f |

### Phase 1: Fundamentos ✅ COMPLETADO

| # | Qué | Status |
|---|-----|--------|
| 7 | `GET /api/v1/properties/keys` | ✅ c435c6f |
| 8 | Frontend usa endpoint real | ✅ 0287902 |
| 9 | CommandRegistry + Cmd+Shift+K | ✅ 69c254b |
| 10 | Renombrar paneles → Cognitive* family | ✅ 33454b3 |
| 11 | Remover tree_rag dead code | ✅ 33454b3 |

### Phase 2: UX Block-Level ✅ COMPLETADO

| # | Qué | Depende | Esfuerzo | Status |
|---|-----|---------|----------|--------|
| 12 | Block Zoom (`?zoom=$blockId`) | — | 0.5 día | ✅ f0b5d76 |
| 13 | Inline+Panel Properties (template) | #7 | 3 días | ✅ 28afeb2 |
| 14 | Quick Capture (CommandRegistry) | #9 | 0.5 día | ✅ builtin |
| 15 | Natural Language Dates V1 | #13 | 1 día | ✅ f0b5d76 |
| 16 | Commandable Transforms (`/task`, `/query`, `/card`) | — | 3h | ✅ 8f6833c |
| 17 | DashboardLayout + PanelVisibility | #9 | 2 días | ✅ 06e28ad |
| 18 | Cognitive* panel implementations | #10, #11 | 3 días | ✅ 28afeb2 |
| 19 | AgentRun block role | — | 2 días | ✅ 06e28ad |
| 20 | SavedView block role | #19 | 2 días | ✅ ses_15c8a3d |

### Phase 3: Infra + Avanzado ✅ COMPLETADO

| # | Qué | Depende | Esfuerzo | Status |
|---|-----|---------|----------|--------|
| 21 | Session cache V1 (dedup api-client) | — | 1 día | ✅ 28afeb2 |
| 22 | Saved/Recent searches | #20 ✅ | 1 día | ✅ 0dbabeb |
| 23 | Graph Lens V1 (subgraph endpoint) | — | 2 días | ✅ 28afeb2 |
| 24 | StrategySelector traits (determinístico) | — | 3 días | ✅ 35af73a |
| 25 | "Save as View" desde search | #20 ✅ | 0.5 día | ✅ 0dbabeb |
| 26 | StrategySelector WASM + hook | #24 | 2 días | ✅ 2c78bec |
| 27 | Graph Lens V2 (lens buttons) | #23 | 1.5 días | ✅ 2c78bec |

### Phase 4: Re-grill Remedies ✅ COMPLETADO

| # | Qué | Remedio | Status |
|---|-----|---------|--------|
| 28 | Editable Backlinks | Enrichment en handler, filtro DSL, whitelist | ✅ 35af73a |
| 29 | Unlinked Ref Queue | Frontend-only, localStorage, PUT existente | ✅ 35af73a |
| 30 | Template Contracts | Extraer diff de reapply.rs, MCP-only | ✅ 35af73a |
| 31 | Template Doctor | Extender structure_gardener, versioning infra | ✅ f68f631 |

### Diferido (V3+)

- Block Shape Detector
- Intent Search completo (NL→DSL)
- SUNNY Phase 2-5 (telemetry, k-NN, auto-execute)

---

## 4. Dependencias (estado actual)

```
Phase 0 ✅ COMPLETO
Phase 1 ✅ COMPLETO
Phase 2 ✅ COMPLETO
Phase 3 ✅ COMPLETO
Phase 4 ✅ COMPLETO (4/4 items, commit 35af73a + f68f631)
```

---

## 5. Estado Final — TODAS LAS FASES COMPLETADAS

**Phase 0-4 ✅ — 31/31 items completados**

### Pendiente (no-bloqueantes)
- Promover ADR drafts #13, #14, #17 a canónicos
- Aplicar CONTEXT.md patch
- User manual (HTML) → deploy o hosting

### Diferido (V3+)
- Block Shape Detector
- Intent Search completo (NL→DSL)
- SUNNY Phase 2-5 (telemetry, k-NN, auto-execute)
