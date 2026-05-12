# Matriz de cobertura de features propuestas en Rust

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

## 6. UI y workflows

| Feature | Documento fuente | Estado en Rust | Evidencia | Nota |
|---|---|---|---|---|
| Base Leptos/WASM | `rust-reimplementation-proposal.md` | Implementada | `crates/quilt-ui/src/lib.rs` | Base existe |
| Journal view | `quilt-ui-workflows.md` | Implementada | `crates/quilt-ui/src/pages/journal.rs` | Existe |
| Query UI básica | `quilt-ui-workflows.md` | Parcial | `crates/quilt-ui/src/pages/query.rs` | Existe, pero no cubre toda la visión |
| Briefing matutino UI | `quilt-ui-workflows.md` | Parcial | `crates/quilt-ui/src/pages/cognitive/dashboard.rs` | Hay wiring parcial |
| Graph view / cognitive map | `quilt-ui-workflows.md` | Stub | UI no verificada completa | Backend relacionado existe |
| Agent Room | `quilt-ui-workflows.md` | Stub | no verificado | No parece implementado |
| Focus mode con AI | `quilt-ui-workflows.md` | Stub | no verificado | No parece implementado |
| Serendipity notifications UI | `quilt-ui-workflows.md` | No encontrada | — | Backend sí, UI no |
| Auto-organize | `quilt-ui-workflows.md` | No encontrada | — | No verificado |
| Weekly review | `quilt-ui-workflows.md` | No encontrada | — | No verificado |
| Decay monitor UI | `quilt-ui-workflows.md` | No encontrada | — | Backend relacionado existe |

### Conclusión del área

La **UI es la mayor zona de brecha** entre lo propuesto y lo actualmente
verificado en Rust.

---

## 7. Platform

| Feature | Documento fuente | Estado en Rust | Evidencia | Nota |
|---|---|---|---|---|
| Tauri desktop shell | `rust-reimplementation-proposal.md` | Implementada | `crates/quilt-platform/src-tauri/` | Cubierta |
| CLI | idem | Implementada | `crates/quilt-platform/src/cli.rs` | Cubierta |
| Deep links | idem | Implementada | `src-tauri/src/deep_link.rs` | Cubierta |
| File watching | idem | Parcial | crates / wiring parcial | No parece cerrado |
| WASM browser target | idem | Parcial | `crates/quilt-ui/` | Compila, pero no equivale a producto completo |

---

## Resumen final por nivel de cobertura

### Claramente cubierto en backend Rust

- core graph/domain
- query DSL
- MCP server y tools principales
- plugins/hooks
- gran parte del backend cognitive/AI
- sync base
- desktop shell / CLI / deep links

### Parcial

- query UI completa
- morning briefing end-to-end
- file watching
- target WASM como experiencia de producto completa
- integración visible de varias capacidades cognitivas

### No cubierto o no verificado como completo

- E2EE
- Agent Room
- Graph View / Cognitive Map UI
- Focus mode con panel AI completo
- Serendipity notifications UI
- Auto-organize
- Weekly review
- Decay monitor UI

## Conclusión

Si la pregunta es:

> “¿Todas las features propuestas están cubiertas en Rust?”

La respuesta correcta es:

> **No. El backend Rust cubre mucho del núcleo y muchas capacidades avanzadas,
> pero no todas las features propuestas están implementadas end-to-end.**

La deuda principal está en:

- UI cognitiva
- workflows de agentes
- features visibles de producto sobre backend ya existente
- y reconciliación entre visión documental e implementación real
