# Matriz de cobertura de features propuestas en Rust

> **Updated**: 2026-06-01 — Stack migration complete (Leptos→React, CM6→TipTap). Tauri removed per ADR-0005. See [AUDIT_REPORT.md](./AUDIT_REPORT.md) for full audit.
> Estado de referencia: mayo de 2026.
> Fuentes principales: `docs/reversa/plan.md`, `domain.md`,
> `quilt-mcp-agent-capabilities.md`, `quilt-ui-workflows.md`,
> `rust-mcp-ai-deep-dive.md`, `rust-reimplementation-proposal.md`.

## Respuesta ejecutiva

**No, no todas las features propuestas están cubiertas en Rust.**

La situación actual parece ser:

- **Backend Rust**: cobertura alta del core y de gran parte de MCP/query/sync.
- **End-to-end (backend + UI + wiring)**: cobertura parcial.
- **Mayor brecha**: UI cognitiva, workflows de agentes y algunas promesas de
  producto más avanzadas que la implementación real.

---

## 1. Core graph y dominio

| Feature | Documento fuente | Estado en Rust | Evidencia | Nota |
|---|---|---|---|---|
| Block entity | `domain.md` | Implementada | `crates/quilt-domain/src/entities/block.rs` | Núcleo cubierto |
| Page entity | `domain.md` | Implementada | `crates/quilt-domain/src/entities/page.rs` | Cubierta |
| Journal pages | `domain.md` | Implementada | `crates/quilt-domain/src/entities/journal.rs` | Incluye `JournalDay` |
| Tags | `domain.md` | Implementada | `crates/quilt-domain/src/entities/tag.rs` | Cubierta |
| Property system tipado | `domain.md` | Implementada | `crates/quilt-domain/src/properties/` | Fuerte cobertura |
| Task markers | `domain.md` | Implementada | `crates/quilt-domain/src/value_objects/task_marker.rs` | La doc tiene drift de casing |
| Priority A/B/C | `domain.md` | Implementada | `crates/quilt-domain/src/value_objects/priority.rs` | Cubierta |
| Block refs / backlinks | `domain.md` | Implementada | dominio + repositorios | Cubierta |
| UUID inmutable | `domain.md` | Implementada | `crates/quilt-domain/src/value_objects/uuid.rs` | Cubierta |
| Normalización de properties | `domain.md` | Implementada | validadores de properties | La doc no siempre está alineada |

### Conclusión del área

El **core del grafo** sí parece estar claramente cubierto en Rust.

---

## 2. Query DSL

| Feature | Documento fuente | Estado en Rust | Evidencia | Nota |
|---|---|---|---|---|
| Parser formal | `query-dsl-spec.md` | Implementada | `crates/quilt-query/src/parser.rs`, `grammar/` | Cubierta |
| `and/or/not` | `query-dsl-spec.md` | Implementada | `crates/quilt-query/src/executor.rs` | Cubierta |
| Filtro por task | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| Filtro por priority | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| Filtro por page | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| Filtro por property | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| Filtro por tags | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| `between` | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| Full-text search | `query-dsl-spec.md` | Implementada | `crates/quilt-search/` | Cubierta |
| `[[Page Name]]` | `query-dsl-spec.md` | Implementada | parser/AST | Cubierta |
| Time helpers | `query-dsl-spec.md` | Implementada | `time_helpers.rs` | Cubierta |
| `sample` | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| `sort-by` | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| `exists/missing` | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |
| Namespace filter | `query-dsl-spec.md` | Implementada | `executor.rs` | Cubierta |

### Conclusión del área

El **Query DSL backend** parece una de las áreas con mejor cobertura real.

---

## 3. MCP y capacidades de agente

| Feature | Documento fuente | Estado en Rust | Evidencia | Nota |
|---|---|---|---|---|
| MCP server core | `rust-mcp-ai-deep-dive.md` | Implementada | `crates/quilt-mcp/src/server.rs` | Cubierta |
| Query tool | idem | Implementada | `server.rs` | Cubierta |
| Create block tool | idem | Implementada | `server.rs` | Cubierta |
| Search tool | idem | Implementada | `server.rs` | Cubierta |
| Get block tree | idem | Implementada | `server.rs` | Cubierta |
| Get page blocks | idem | Implementada | `server.rs` | Cubierta |
| List pages | idem | Implementada | `server.rs` | Cubierta |
| Get journal | idem | Implementada | `server.rs` | Cubierta |
| Create task | idem | Implementada | `server.rs` | Cubierta |
| Link blocks | idem | Implementada | `server.rs` | Cubierta |
| Backlinks | idem | Implementada | `server.rs` | Cubierta |
| Delete / restore / recycle bin | idem | Implementada | `server.rs` | Cubierta |
| Resources MCP | idem | Implementada | `server.rs` | Cubierta |
| Notifications | idem | Implementada | `crates/quilt-mcp/src/notifications.rs` | Cubierta |
| Plugin system | `quilt-mcp-agent-capabilities.md` | Implementada | `crates/quilt-mcp/src/plugin/` | Cubierta |
| Hook system | `quilt-mcp-agent-capabilities.md` | Implementada | `crates/quilt-mcp/src/hooks/` | Cubierta |
| Git extension | `quilt-mcp-agent-capabilities.md` | Implementada | `crates/quilt-git-extension/` | Cubierta |

### Conclusión del área

La capa **MCP** está ampliamente cubierta en backend Rust.

---

## 4. Cognitive / AI

| Feature | Documento fuente | Estado en Rust | Evidencia | Nota |
|---|---|---|---|---|
| CognitiveMirror | `quilt-mcp-agent-capabilities.md` | Implementada | `crates/quilt-cognitive/src/cognitive_mirror/` | Backend existe |
| SerendipityEngine | idem | Implementada | `crates/quilt-cognitive/src/serendipity/` | Backend existe |
| ArgumentCartographer | idem | Implementada | `crates/quilt-cognitive/src/argument_cartographer/` | Backend existe |
| MentalModelGardener | idem | Implementada | `crates/quilt-cognitive/src/mental_model_gardener/` | Backend existe |
| CounterfactualExplorer | idem | Implementada | `crates/quilt-cognitive/src/counterfactual_explorer/` | Backend existe |
| KnowledgeEvolutionTracker | idem | Implementada | `crates/quilt-cognitive/src/knowledge_evolution/` | Backend existe |
| AgentMemory | idem | Implementada | `crates/quilt-cognitive/src/agent_memory/` | Backend existe |
| TreeRAG | idem | Implementada | `crates/quilt-cognitive/src/tree_rag/` | Backend existe |
| MorningBriefing backend | `quilt-ui-workflows.md` | Implementada | `crates/quilt-cognitive/src/morning_briefing/` | DTO y lógica existen |
| TaskScheduler | `quilt-mcp-agent-capabilities.md` | Implementada | `crates/quilt-cognitive/src/scheduler/` | Cubierta |
| Multi-agent perspectives / debate | `quilt-mcp-agent-capabilities.md` | Stub | infraestructura parcial | No parece cerrada end-to-end |

### Conclusión del área

El **backend cognitivo** parece fuerte. La debilidad está en su traducción a
producto visible y workflows completos.

---

## 5. Sync

| Feature | Documento fuente | Estado en Rust | Evidencia | Nota |
|---|---|---|---|---|
| Motor de sync | `rust-reimplementation-proposal.md` | Implementada | `crates/quilt-sync/src/crdt.rs` | Existe, aunque la doc sugiere una visión más ambiciosa |
| Resolución LWW | idem | Implementada | `crdt.rs` | Cubierta |
| PreserveBoth | idem | Implementada | `crdt.rs` | Cubierta |
| Manual resolution | idem | Implementada | `crdt.rs` | Cubierta |
| Offline queue / WAL | idem | Implementada | `crates/quilt-sync/src/offline.rs` | Cubierta |
| Sync state management | idem | Implementada | `crates/quilt-sync/src/state.rs` | Cubierta |
| Transport abstraction | idem | Implementada | `crates/quilt-sync/src/transport.rs` | Cubierta |
| E2EE | `domain.md` | No encontrada | — | No verificada |

### Conclusión del área

Hay **sync real**, pero no puede afirmarse que la implementación coincida por
completo con toda la promesa arquitectónica histórica.

---

## 6. UI y workflows (React/TypeScript)

| Feature | Documento fuente | Estado | Evidencia | Nota |
|---|---|---|---|---|
| Base React/TypeScript | ADR-0006 | ✅ Completa | `ui/` directorio raíz | Migrada desde Leptos|
| Journal view | ADR-0006 | ✅ Completa | React + MCP API | Wired via MCP|
| Query UI | ADR-0006 | ✅ Completa | React + MCP API | Query DSL via MCP|
| Search UI | ADR-0006 | ✅ Completa | React + FTS5 | Full-text search via MCP|
| Auth | ADR-0006 | ✅ Completa | React + JWT | JWT-based auth|
| E2E Tests (Playwright) | ADR-0006 | ✅ Completa | `ui/e2e/` | Playwright suite para React|
| Outliner / Block Editor | ADR-0006 | ✅ Completa | TipTap editor | Migrado de CM6|
| Page Editor | ADR-0006 | ✅ Completa | React + MCP | Create/edit via MCP|
| Cognitive Dashboard | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|
| Agent Room | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|
| Graph view / cognitive map | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|
| Focus mode con AI | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|
| Serendipity notifications UI | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|
| Auto-organize | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|
| Weekly review | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|
| Decay monitor UI | ADR-0001 | ❌ No implementada | — | Not in scope per ADR-0001|

### Conclusión del área

La **UI base está completa** con React/TypeScript + TipTap. Las features
cognitivas de UI quedaron fuera de scope por ADR-0001. El backend cognitivo
existe y está disponible vía MCP para clientes externos.

---

## 7. React Migration (ADR-0006)

| Feature | Estado | Evidencia | Nota |
|---|---|---|---|
| Leptos→React migration | ✅ Completa | `ui/` directorio raíz | Todo el frontend migrado |
| CM6→TipTap editor | ✅ Completa | `ui/` editor components | Editor de bloques funcional |
| Real API via MCP | ✅ Completa | React → MCP client | Sin mocks |
| Auth (JWT) | ✅ Completa | React auth module | JWT-based |
| E2E tests (Playwright) | ✅ Completa | `ui/e2e/` | Suite de pruebas |
| Build/deploy pipeline | ✅ Completa | CI config | React build + deploy |

**Nota**: La migración a React fue una decisión arquitectónica (ADR-0006) que
reemplazó el stack Leptos/WASM + Tauri por React/TypeScript + MCP directo.

---

## 8. Platform

| Feature | Documento fuente | Estado | Evidencia | Nota |
|---|---|---|---|---|
| Tauri desktop shell | `rust-reimplementation-proposal.md` | ❌ Removida | ADR-0005 | Eliminada por ADR-0005 |
| CLI | idem | ✅ Implementada | `crates/quilt-platform/src/cli.rs` | Cubierta |
| Deep links | idem | ❌ Removida | ADR-0005 | Eliminado con Tauri |
| File watching | idem | Parcial | crates / wiring parcial | No parece cerrado |
| Web SPA (React) | ADR-0006 | ✅ Completa | `ui/` | Reemplaza a Tauri + WASM |

---

## Resumen final por nivel de cobertura

### Claramente cubierto en backend Rust

- core graph/domain
- query DSL
- MCP server y tools principales
- plugins/hooks
- gran parte del backend cognitive/AI
- sync base
- CLI

### Claramente cubierto en frontend React

- UI base (React/TypeScript + TipTap)
- Journal view, Page List, Search, Query
- Auth (JWT)
- E2E tests (Playwright)
- Outliner / Block Editor / Page Editor

### Parcial

- morning briefing end-to-end
- file watching
- integración visible de varias capacidades cognitivas en UI

### No cubierto o fuera de scope

- E2EE
- Cognitive UI features (fuera de scope per ADR-0001)
- Tauri desktop shell / deep links (removidos per ADR-0005)

## Conclusión

Si la pregunta es:

> “¿Todas las features propuestas están cubiertas?”

La respuesta correcta es:

> **El stack completo (Rust backend + React frontend) cubre el núcleo del
> producto. La UI cognitiva quedó fuera de scope por ADR-0001. Tauri fue
> removido por ADR-0005.**

La deuda principal está en:

- integración de engines cognitivos en la UI React
- workflows de agentes end-to-end
- sync real end-to-end (transporte + UI)
- y reconciliación entre visión documental e implementación real
