# Spec Impact Matrix — Quilt

> **Fecha**: 2026-05-12
> **Propósito**: Matriz de impacto que mapea features/capabilities a módulos implementados en Rust.
> **Canon**: Este documento es la fuente canonical para Quilt (Rust). Para el schema de Logseq (Clojure original),
> ver `docs/reversa/erd.md` y `docs/reversa/data-dictionary.md`.

---

## 1. Features/Capabilities

| ID | Feature | Descripción | Status |
|----|---------|-------------|--------|
| F01 | Block Editor | Crear, editar, eliminar bloques con formato rico | ✅ Implemented |
| F02 | Outliner Tree | Estructura jerárquica de bloques (indent, move, collapse) | ✅ Implemented |
| F03 | Page Management | Crear, renombrar, eliminar, archivar páginas | ✅ Implemented |
| F04 | Journal | Notas diarias automáticas con calendario | ✅ Implemented |
| F05 | Query DSL | Lenguaje de consultas sobre datos | ✅ Implemented |
| F06 | Full-text Search | Búsqueda de contenido via FTS5 | ✅ Implemented |
| F07 | Graph Visualization | Vista de grafo de conocimiento | 🔲 Planned (v2) |
| F08 | Tags & Properties | Sistema de tagging y propiedades custom | ✅ Implemented |
| F09 | Block References | Referencias bidireccionales `{{uuid}}` | ✅ Implemented |
| F10 | Page References | Links a páginas `[[page]]` | ✅ Implemented |
| F11 | Timestamps | Scheduled, deadline, start-time, repetition | ✅ Implemented |
| F12 | Task Management | Estados de tarea (NOW/LATER/TODO/DONE/CANCELLED) | ✅ Implemented |
| F13 | Asset Embedding | Imágenes, PDFs, videos embebidos | ✅ Implemented |
| F14 | Export/Publishing | Exportar a Markdown, HTML, sitio estático | 🔲 Planned |
| F15 | Git Sync | Sincronización via Git | 🔲 Planned |
| F16 | Multi-graph | Múltiples grafos/repositorios | 🔲 Planned |
| F17 | Plugins API | Extensibilidad via plugins | 🔲 Planned |
| F18 | Themes | Sistema de temas y personalización CSS | 🔲 Planned |
| F19 | Mobile | Apps iOS/Android | 🔲 Planned |
| F20 | PDF Annotation | Anotación de PDFs | 🔲 Planned |

**Status Legend**: ✅ = Implemented in current codebase | 🔲 = Planned for future | 🟡 = Partial/In Progress

---

## 2. Módulos Rust (DDD Crates)

| ID | Módulo | Crate | Responsabilidad |
|----|--------|-------|-----------------|
| M01 | Domain | `quilt-domain` | Entidades puras, value objects, traits de repositorio |
| M02 | Application | `quilt-application` | Command/query handlers, casos de uso |
| M03 | Infrastructure | `quilt-infrastructure` | Implementaciones SQLite, repositorios concretos |
| M04 | Query | `quilt-query` | Parser PEG + executor para Query DSL |
| M05 | Search | `quilt-search` | FTS5 indexing y búsqueda |
| M06 | Sync | `quilt-sync` | Motor de sincronización (LWW actual, CRDT planeado) |
| M07 | MCP | `quilt-mcp` | Server MCP, tools, resources, notifications |
| M08 | Platform | `quilt-platform` | Tauri desktop shell, CLI |

---

## 3. Matriz de Impacto

### Simbología

| Símbolo | Impacto | Descripción |
|---------|---------|-------------|
| 🟢 H | **HIGH** | Módulo es core/implements esta feature directamente |
| 🟡 M | **MEDIUM** | Módulo contribute o es вспомогательный |
| 🔴 L | **LOW** | Interacción mínima |
| — | **N/A** | Sin relación directa |
| ✅ | **Implemented** | Feature completamente implementada |
| 🔲 | **Planned** | Feature para futuro (v2+) |

---

### Matriz Feature → Módulo

| | M01 Domain | M02 Application | M03 Infrastructure | M04 Query | M05 Search | M06 Sync | M07 MCP | M08 Platform |
|---|-----|-----|-----|-----|-----|-----|-----|-----|
| **F01** Block Editor | 🟢H | 🟢H | 🟢H | — | — | — | 🟡M | — |
| **F02** Outliner Tree | 🟢H | 🟡M | 🟢H | — | — | — | — | — |
| **F03** Page Management | 🟢H | 🟢H | 🟢H | — | — | — | 🟡M | — |
| **F04** Journal | 🟢H | 🟢H | 🟢H | — | — | — | 🟡M | — |
| **F05** Query DSL | 🟡M | 🟡M | — | 🟢H | — | — | — | — |
| **F06** Full-text Search | — | — | — | — | 🟢H | — | — | — |
| **F07** Graph Viz | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| **F08** Tags & Properties | 🟢H | 🟡M | 🟢H | — | — | — | — | — |
| **F09** Block References | 🟢H | 🟡M | 🟢H | — | — | — | 🟡M | — |
| **F10** Page References | 🟢H | 🟡M | 🟢H | — | — | — | 🟡M | — |
| **F11** Timestamps | 🟢H | 🟡M | 🟢H | — | — | — | — | — |
| **F12** Task Management | 🟢H | 🟡M | 🟢H | — | — | — | 🟡M | — |
| **F13** Asset Embedding | 🟢H | 🟡M | 🟢H | — | — | — | — | — |
| **F14** Export/Publishing | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| **F15** Git Sync | 🔲 | 🔲 | 🔲 | — | — | 🔲 | — | 🔲 |
| **F16** Multi-graph | 🔲 | 🔲 | 🔲 | — | — | 🔲 | — | 🔲 |
| **F17** Plugins API | 🔲 | 🔲 | 🔲 | — | — | — | 🔲 | 🔲 |
| **F18** Themes | 🔲 | 🔲 | 🔲 | — | — | — | 🔲 | 🔲 |
| **F19** Mobile | 🔲 | 🔲 | 🔲 | — | — | 🔲 | 🔲 | 🔲 |
| **F20** PDF Annotation | 🔲 | 🔲 | 🔲 | — | — | — | — | 🔲 |

---

## 4. Task Markers Canonical Reference

> **Status**: ✅ Fully implemented across all layers

### Canonical Values

| Marker | Significado | Estado | Notes |
|--------|-------------|--------|-------|
| `NOW` | En progreso | Activo | marker = `block/marker` en SQLite |
| `LATER` | Planificado | Pendiente | |
| `TODO` | Por hacer | Pendiente | |
| `DONE` | Completado | Completo | Estado terminal |
| `CANCELLED` | Cancelado | Cancelado | Estado terminal |

### Implementation References

- **Domain**: `quilt-domain/src/value_objects/task_marker.rs` — enum `TaskMarker`
- **Infrastructure**: `marker TEXT` column en `blocks` table
- **Query**: `(task marker*)` filter operator
- **MCP**: `logseq_create_task` tool usa `TaskMarker::Todo`

### Normalization Rules

1. Task markers are stored as uppercase strings: `NOW`, `LATER`, `TODO`, `DONE`, `CANCELLED`
2. Only one active marker per block (no multiple markers)
3. `DONE` and `CANCELLED` are terminal states

---

## 5. Property Normalization Rules

> **Status**: ✅ Documented, varies slightly by context

### Canonical Rules (Logseq Clojure)

- Property names normalized: lowercase, `/` → `-`, spaces → `-`, `_` → `-`
- Example: `Block UUID` → `block-uuid`

### Quilt/Rust Implementation

- Properties stored as JSON in `properties BLOB` column
- Key normalization happens at parse time (graph-parser)
- Query DSL handles property access via `json_extract(properties, '$.key')`

### Validation Rules

- Property names cannot be empty
- Closed-value properties (markers, priority) have restricted values
- Custom properties: no restrictions beyond non-empty name

---

## 6. Sync Strategy — Current vs Planned

> **Critical Gap**: Spec says Loro CRDT, impl uses custom LWW

### Current Implementation (LWW)

| Aspect | Status | Location |
|--------|--------|----------|
| Custom LWW sync | ✅ Implemented | `quilt-sync/src/crdt.rs` |
| Offline queue (WAL) | ✅ Implemented | `quilt-sync/src/offline.rs` |
| Conflict detection | ✅ Implemented | Via LWW timestamps |

### Planned (CRDT with Loro)

| Aspect | Status | Notes |
|--------|--------|-------|
| Loro CRDT integration | 🔲 Planned | `loro = "0.2"` in Cargo.toml (unused) |
| True conflict-free merge | 🔲 Planned | Would replace LWW |
| Collaborative editing | 🔲 Planned | Requires true CRDT |

### Decision Required

The spec/design documents describe Loro CRDT as the sync strategy, but the actual implementation uses a simpler Last-Write-Wins approach with timestamps. This is a known architectural mismatch that should be resolved before building more sync infrastructure.

**Options**:
1. **Adopt Loro**: Integrate `loro` crate per design spec
2. **Formalize LWW**: Document LWW as the intentional strategy with rationale

See: `docs/reversa/_reversa_sdd/LLM_FIRST_ROADMAP.md` §"Sync Strategy Clarification"

---

## 7. Known Gaps and Issues

| Gap | Severity | Description | tracked |
|-----|----------|-------------|---------|
| Sync strategy mismatch (LWW vs CRDT) | 🔴 HIGH | Spec says Loro, impl uses custom LWW | LLM_FIRST_ROADMAP.md |
| Cognitive engines not wired | 🔴 HIGH | Engines exist but `None` in MCP server | LLM_FIRST_ROADMAP.md |
| Missing MCP Resources | 🟡 MED | `logseq://cognitive/*` hierarchy not implemented | LLM_FIRST_ROADMAP.md |
| Search retry policy | 🟡 MED | No retry with backoff in search | LLM_FIRST_ROADMAP.md |
| Multi-graph support | 🟡 MED | Single graph only currently | roadmap.md |

---

## 8. Heatmap de Complejidad de Cambios

| Módulo | Features afetadas | Risk Score |
|--------|-------------------|------------|
| M03 (Infrastructure) | 12 | 🔴 HIGH |
| M01 (Domain) | 10 | 🔴 HIGH |
| M07 (MCP) | 6 | 🟡 MEDIUM |
| M04 (Query) | 1 | 🟢 LOW |
| M05 (Search) | 1 | 🟢 LOW |
| M06 (Sync) | 1 | 🟡 MEDIUM (LWW vs CRDT) |

---

## 9. Vistas por Módulo

### M01 - quilt-domain 🟢

| Feature | Impacto | Razón |
|---------|---------|-------|
| F01 Block Editor | 🟢H | Block entity core |
| F02 Outliner Tree | 🟢H | Tree structure in domain |
| F03 Page Management | 🟢H | Page entity core |
| F04 Journal | 🟢H | JournalPage specialization |
| F08 Tags & Properties | 🟢H | PropertyValue types |
| F09 Block References | 🟢H | Ref entity |
| F10 Page References | 🟢H | Part of Block/Page |
| F11 Timestamps | 🟢H | Scheduled, deadline value objects |
| F12 Task Management | 🟢H | TaskMarker enum |

### M07 - quilt-mcp 🟢

| Feature | Impacto | Razón |
|---------|---------|-------|
| F01 Block Editor | 🟡M | `logseq_create_block`, `logseq_update_block` tools |
| F03 Page Management | 🟡M | `logseq_create_page`, `logseq_rename_page` tools |
| F04 Journal | 🟡M | `logseq_get_journal` tool |
| F09 Block References | 🟡M | `logseq_link_blocks`, `logseq_get_backlinks` tools |
| F10 Page References | 🟡M | Via query and link tools |
| F12 Task Management | 🟡M | `logseq_create_task` tool |

---

*Document generated as part of SDD reconcile-foundation-docs - 2026-05-12*
