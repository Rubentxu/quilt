# Quilt — Action Roadmap

> Generated: 2026-06-07
> Sources: auto-grill 30 cycles, ux-workflow-portfolio-analysis.md, 7 ADR drafts, CONTEXT.md patch

---

## 0. Estado Actual

### Funciona
- Backend: Axum + SQLite + FTS5 + MCP tools + property CRUD
- Frontend: React 19 + TipTap + sidebar + block editor + search modal + templates
- WASM: quilt-core compila a wasm32-unknown-unknown
- Dev: `just dev` — hot reload completo

### Roto (P0)
- `/` devuelve null (sin redirect a journal)
- QueryPage envía `QueryAst` inválido via `as any`
- API client llama a rutas no montadas (SSE, analysis, schema-pack)
- Graph dark mode detecta `.dark` pero la app usa `data-theme`
- `cognitive.rs` no compila — 4 AppState fields faltantes
- CutBlock no tiene undo
- **`blockType` no persiste** — el frontend envía `blockType` pero el backend (`UpdateBlockRequest`) no lo tiene. 11 slash commands (`/h1`, `/code`, `/quote`, etc.) parecen funcionar en sesión pero se pierden al recargar

### Slash commands `/` — YA FUNCIONAN (29 registrados)

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
| Block types | `/text` `/h1`-`/h3` `/bullet` `/numbered` `/todo` `/quote` `/code` `/divider` `/image` | `api.updateBlock({ blockType })` |

**Lo que FALTA** (Q019 — Commandable Transforms):
- `/task` → agrega `type:: task` (rol)
- `/query` → agrega `type:: query` + `dsl::`
- `/card` → agrega `card-shape::` (template-driven)

La infraestructura (`defaultRegistry.register()`) ya soporta agregar estos sin tocar BlockRow.

**⚠️ BUG: `blockType` no persiste** — los 11 comandos de Block Types (`/h1`-`/h3`, `/code`, `/quote`, etc.) envían `api.updateBlock({ blockType })` pero el backend no tiene `blockType` en `UpdateBlockRequest`. Funciona en sesión (state local) pero se pierde al recargar. Fix: Phase 0 #6.

### Bloqueantes para features nuevas
- `GET /api/v1/properties/keys` — frontend hardcodea property keys
- CommandRegistry (Cmd+Shift+K) — palette separada del slash menu
- Templates no declaran layout de properties (inline vs panel)

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

### De esta sesión — candidatos a promover

| Target | Draft | Decisión | Confianza |
|--------|-------|----------|-----------|
| 0011 | DRAFT-dashboard-layout-no-work-modes.md | DashboardLayout como presets de paneles, no Work Modes | Alta |
| 0012 | DRAFT-command-registry-mcp-dispatch.md | React context + Cmd+Shift+K, dispatch MCP híbrido | Alta |
| 0013 | DRAFT-cognitive-panel-family-namespace.md | Cognitive* bajo namespace `cognitivo::` | Alta |
| 0014 | DRAFT-property-schema-endpoint.md | GET /properties/keys con cursor pagination | Media |
| 0015 | DRAFT-agent-run-block-role.md | AgentRun = `type:: agent-run` block role | Alta |
| 0016 | DRAFT-saved-view-block-role.md | SavedView = `type:: view` + `data-source::` | Alta |
| 0017 | DRAFT-strategy-selector-trait-contract.md | StrategySelector + StrategyScorer traits en quilt-core (WASM) | Alta |

---

## 2. CONTEXT.md

Aplicar patch desde `docs/grill/.state/CONTEXT.patch.md`:

**Agregar**: AgentRun, SavedView, DashboardLayout, CommandRegistry, ViewContainer, Cognitive* family, StrategySelector

**Modificar**: Rol (agregar `agent-run`, `insight`), View (agregar `data-source::`), Query embebida (referencia a view)

**Eliminar**: Serendipity Feed, Agent Workbench, ConnectionType::Semantic/Content

---

## 3. Fases de Implementación

### Phase 0: P0 Fixes (1-2 días, sin dependencias)

| # | Qué | Ciclo | Esfuerzo |
|---|-----|-------|----------|
| 1 | Home redirect `/` → journal de hoy | Q030 | 1h |
| 2 | Eliminar QueryPage roto | Q031 | 1h |
| 3 | Fix graph dark mode (`data-theme`) | Q033 | 30m |
| 4 | Alinear API client con rutas montadas | Q032 | 2h |
| 5 | CutBlock + UndoManager (Cmd+Z) | Q021 | 3h |
| 6 | **Backend: agregar `blockType` a `BlockDto` + `UpdateBlockRequest`** | — | 3h |

**Check**: `just dev` sin errores en consola, graph en dark mode, cut+undo funciona.

### Phase 1: Fundamentos (3-5 días)

| # | Qué | Depende | Esfuerzo |
|---|-----|---------|----------|
| 7 | `GET /api/v1/properties/keys` | — | 1 día |
| 8 | Frontend usa endpoint real | #7 | 2h |
| 9 | CommandRegistry + Cmd+Shift+K | — | 1.5 días |
| 10 | Renombrar paneles → Cognitive* family | cargo check | 2h |
| 11 | Remover tree_rag dead code | #10 | 1h |

**Check**: properties desde API, Cmd+Shift+K abre palette, `cargo check` limpio.

### Phase 2: UX Block-Level (1-2 semanas)

| # | Qué | Depende | Esfuerzo |
|---|-----|---------|----------|
| 12 | Block Zoom (`?zoom=$blockId`) | — | 0.5 día |
| 13 | Inline+Panel Properties (template) | #7 | 3 días |
| 14 | Quick Capture (CommandRegistry) | #9 | 0.5 día |
| 15 | Natural Language Dates V1 | #13 | 1 día |
| 16 | Commandable Transforms (`/task`, `/query`, `/card`) — 3 handlers nuevos en registry existente | — | 3h |
| 17 | DashboardLayout + PanelVisibility | #9 | 2 días |
| 18 | Cognitive* panel implementations | #10, #11 | 3 días |
| 19 | AgentRun block role | — | 2 días |
| 20 | SavedView block role | #19 | 2 días |

### Phase 3: Infra + Avanzado (2-3 semanas)

| # | Qué | Depende | Esfuerzo |
|---|-----|---------|----------|
| 21 | Session cache V1 (dedup api-client) | — | 1 día |
| 22 | Saved/Recent searches | #20 | 1 día |
| 23 | Graph Lens V1 (subgraph endpoint) | — | 2 días |
| 24 | StrategySelector traits (determinístico) | — | 3 días |
| 25 | "Save as View" desde search | #20 | 0.5 día |
| 26 | StrategySelector WASM + hook | #24 | 2 días |
| 27 | Graph Lens V2 (lens buttons) | #23 | 1.5 días |

### Phase 4: Re-grill Remedies

| # | Qué | Remedio |
|---|-----|---------|
| 28 | Editable Backlinks | Enrichment en handler, filtro DSL, whitelist |
| 29 | Unlinked Ref Queue | Frontend-only, localStorage, PUT existente |
| 30 | Template Contracts | Extraer diff de reapply.rs, MCP-only |
| 31 | Template Doctor | Extender structure_gardener, versioning infra |

### Diferido (V3+)

- Block Shape Detector
- Intent Search completo (NL→DSL)
- SUNNY Phase 2-5 (telemetry, k-NN, auto-execute)

---

## 4. Dependencias

```
Phase 0 ─── sin deps, hacer ya
  │  #6 blockType backend es prerequisite para que los slash commands persistan
  │
  ▼
Phase 1
  #7 properties ──► #8 frontend
  #9 command-reg ──► #14 quick-capture
  │               ──► #17 dashboard
  #10 cognitive ──► #11 tree_rag ──► #18 panels
  │
  ▼
Phase 2
  #12 zoom (indep.)
  #7 ────────► #13 inline-props ──► #15 NL dates
  #19 agent-run ► #20 saved-view ──► #22 saved searches
  │                                  ──► #25 save-as-view
  #23 graph V1 ──► #27 graph V2
  #24 strategy ──► #26 WASM hook
  #21 cache (indep.)
  │
  ▼
Phase 4: remedies (28-31)
```

---

## 5. Próximos Pasos Inmediatos

1. **Revisar 7 drafts** → promover a 0011-0017 o pedir cambios
2. **Aplicar CONTEXT.md patch**
3. **Fix `cognitive.rs`** → desbloquea Phase 1
4. **Empezar Phase 0** → 5 fixes sin dependencias
