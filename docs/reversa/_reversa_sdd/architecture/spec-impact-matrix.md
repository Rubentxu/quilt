# Spec Impact Matrix — Logseq

> **Escala**: 🟢 CONFIRMADO | 🟡 INFERIDO | 🔴 LACUNA
> **Fecha**: 2026-05-02
> **Metodología**: Matriz de impacto feature → módulo

---

## Definición de Features/Capabilities

| ID | Feature | Descripción |
|----|---------|-------------|
| F01 | Block Editor | Crear, editar, eliminar bloques con formato rico |
| F02 | Outliner Tree | Estructura jerárquica de bloques (indent, move, collapse) |
| F03 | Page Management | Crear, renombrar, eliminar, archivar páginas |
| F04 | Journal | Notas diarias automáticas con calendario |
| F05 | Query DSL | Lenguaje de consultas sobre datos |
| F06 | Full-text Search | Búsqueda de contenido en tiempo real |
| F07 | Graph Visualization | Vista de grafo de conocimiento |
| F08 | Tags & Properties | Sistema de tagging y propiedades custom |
| F09 | Block References | Referencias bidireccionales `{{uuid}}` |
| F10 | Page References | Links a páginas `[[page]]` |
| F11 | Timestamps | Scheduled, deadline, start-time, repetition |
| F12 | Task Management | Estados de tarea (todo, doing, done) con prioridades |
| F13 | Asset Embedding | Imágenes, PDFs, videos embebidos |
| F14 | Export/Publishing | Exportar a Markdown, HTML, sitio estático |
| F15 | Git Sync | Sincronización via Git |
| F16 | Multi-graph | Múltiples grafos/repositorios |
| F17 | Plugins API | Extensibilidad via plugins |
| F18 | Themes | Sistema de temas y personalización CSS |
| F19 | Mobile | Apps iOS/Android |
| F20 | PDF Annotation | Anotación de PDFs |

---

## Módulos del sistema

| ID | Módulo | Ruta | Responsabilidad |
|----|--------|------|-----------------|
| M01 | frontend/components | `src/main/frontend/components/` | UI components (Rum/React) |
| M02 | frontend/state | `src/main/frontend/state.cljs` | Global UI state (atoms) |
| M03 | frontend/handler | `src/main/frontend/handler/` | Event handlers y event loop |
| M04 | frontend/db | `src/main/frontend/db/` | DataScript access, queries, transactions |
| M05 | frontend/fs | `src/main/frontend/fs/` | File system abstraction |
| M06 | frontend/format | `src/main/frontend/format/` | Markdown/Org parsing (Mldoc) |
| M07 | frontend/search | `src/main/frontend/search/` | Search engine (agency pattern) |
| M08 | graph-parser | `deps/graph-parser/` | File → DB transformation |
| M09 | outliner | `deps/outliner/` | Tree operations para blocks |
| M10 | electron | `src/electron/` | Desktop app wrapper |
| M11 | mobile | `src/main/mobile/` | Mobile wrapper (Capacitor) |
| M12 | plugins-api | `src/main/logseq/` | Plugin API pública |
| M13 | publishing | `src/main/frontend/publishing/` | Site generation |

---

## Matriz de Impacto

### Simbología

| Símbolo | Impacto | Descripción |
|---------|---------|-------------|
| 🟢 H | **HIGH** | Módulo es core para esta feature |
| 🟡 M | **MEDIUM** | Módulo contribuye o es вспомогательный |
| 🔴 L | **LOW** | Módulo tiene interacción mínima |
| — | **N/A** | Sin relación directa |

---

### Matriz Feature → Módulo

| | M01 | M02 | M03 | M04 | M05 | M06 | M07 | M08 | M09 | M10 | M11 | M12 | M13 |
|---|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|
| **F01** Block Editor | 🟢H | 🟢H | 🟢H | 🟡M | — | 🟡M | — | — | 🟡M | — | — | — | — |
| **F02** Outliner Tree | 🟢H | 🟢H | 🟡M | 🟢H | — | — | — | — | 🟢H | — | — | — | — |
| **F03** Page Management | 🟢H | 🟡M | 🟢H | 🟢H | 🟡M | 🟡M | — | 🟡M | 🟡M | — | — | — | — |
| **F04** Journal | 🟢H | 🟡M | 🟡M | 🟢H | 🟡M | 🟡M | — | 🟡M | 🟡M | — | — | — | — |
| **F05** Query DSL | 🟡M | 🟡M | — | 🟢H | — | — | 🟡M | — | — | — | — | — | — |
| **F06** Full-text Search | 🟡M | 🟡M | 🟡M | 🟡M | — | — | 🟢H | — | — | — | — | 🟡M | — |
| **F07** Graph Visualization | 🟢H | 🟡M | — | 🟡M | — | — | — | — | — | — | — | — | — |
| **F08** Tags & Properties | 🟡M | — | 🟡M | 🟢H | — | 🟡M | — | 🟢H | — | — | — | — | — |
| **F09** Block References | 🟢H | 🟡M | 🟡M | 🟢H | — | 🟡M | 🟡M | 🟡M | 🟡M | — | — | — | — |
| **F10** Page References | 🟢H | 🟡M | 🟡M | 🟢H | — | 🟡M | 🟡M | 🟡M | 🟡M | — | — | — | — |
| **F11** Timestamps | 🟢H | — | 🟡M | 🟢H | — | 🟡M | — | 🟢H | — | — | — | — | — |
| **F12** Task Management | 🟢H | — | 🟡M | 🟢H | — | 🟡M | — | 🟢H | 🟡M | — | — | — | — |
| **F13** Asset Embedding | 🟢H | — | 🟡M | 🟡M | 🟢H | — | — | — | — | — | — | — | — |
| **F14** Export/Publishing | 🟡M | — | — | 🟡M | 🟡M | 🟡M | — | — | — | — | — | — | 🟢H |
| **F15** Git Sync | — | — | 🟡M | 🟡M | 🟢H | — | — | 🟢H | — | 🟡M | — | — | — |
| **F16** Multi-graph | 🟡M | 🟢H | 🟢H | 🟢H | 🟢H | — | — | 🟡M | — | 🟡M | — | — | — |
| **F17** Plugins API | 🟡M | 🟡M | 🟡M | 🟡M | — | — | 🟡M | — | — | — | — | 🟢H | — |
| **F18** Themes | 🟢H | 🟡M | — | — | — | — | — | — | — | — | — | 🟡M | — |
| **F19** Mobile | 🟢H | 🟡M | 🟡M | 🟡M | 🟡M | 🟡M | — | — | — | — | 🟢H | 🟡M | — |
| **F20** PDF Annotation | 🟡M | — | — | 🟡M | 🟢H | — | — | — | — | — | — | — | — |

---

## Vista por Módulo

### M01 - frontend/components 🟢 (HIGH en 11 features)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F01 Block Editor | 🟢H | Editor component es el core |
| F02 Outliner Tree | 🟢H | Renderizado de árbol |
| F03 Page Management | 🟢H | Page component es UI principal |
| F04 Journal | 🟢H | Journal component específico |
| F07 Graph Visualization | 🟢H | Global graph component |
| F09 Block References | 🟢H | Renderizado de refs `{{uuid}}` |
| F10 Page References | 🟢H | Renderizado de `[[page]]` |
| F11 Timestamps | 🟢H | Timestamp component |
| F12 Task Management | 🟢H | Marker/checkbox UI |
| F13 Asset Embedding | 🟢H | Asset container, resizable-image |
| F19 Mobile | 🟢H | Mobile UI components |

---

### M02 - frontend/state 🟢 (HIGH en 2 features)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F01 Block Editor | 🟢H | Editor state (cursor, content, mode) |
| F16 Multi-graph | 🟢H | Global repo/route state |

---

### M03 - frontend/handler 🟢 (HIGH en 2 features)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F03 Page Management | 🟢H | Event handlers para pages |
| F16 Multi-graph | 🟢H | Graph switch handlers |

---

### M04 - frontend/db 🟢 (HIGH en 9 features)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F02 Outliner Tree | 🟢H | DataScript transactions |
| F03 Page Management | 🟢H | Page queries y transactions |
| F04 Journal | 🟢H | Journal page queries |
| F05 Query DSL | 🟢H | DSL parser y executor |
| F08 Tags & Properties | 🟢H | Tags y properties schema |
| F09 Block References | 🟢H | Refs storage y resolution |
| F10 Page References | 🟢H | Page refs storage |
| F11 Timestamps | 🟢H | Timestamp fields schema |
| F12 Task Management | 🟢H | Marker/properties schema |

---

### M05 - frontend/fs 🟢 (HIGH en 2 features)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F13 Asset Embedding | 🟢H | Asset file reading |
| F15 Git Sync | 🟢H | File operations para sync |
| F16 Multi-graph | 🟢H | Multi-repo file access |

---

### M08 - graph-parser 🟢 (HIGH en 3 features)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F08 Tags & Properties | 🟢H | Property extraction |
| F11 Timestamps | 🟢H | Timestamp parsing |
| F12 Task Management | 🟢H | Marker/property detection |
| F15 Git Sync | 🟢H | File content parsing |

---

### M09 - outliner 🟢 (HIGH en 2 features)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F02 Outliner Tree | 🟢H | Tree operations core |

---

### M12 - plugins-api 🟢 (HIGH en 1 feature)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F17 Plugins API | 🟢H | API pública es el core |

---

### M13 - publishing 🟢 (HIGH en 1 feature)

| Feature | Impacto | Razón |
|---------|---------|-------|
| F14 Export/Publishing | 🟢H | Site generation core |

---

## Dependencias de implementación

### Features que atraviesan múltiples módulos

| Feature | Módulos atravessados | Complejidad |
|---------|---------------------|-------------|
| F01 Block Editor | M01, M02, M03, M04, M06, M09 | 🟡 Alta |
| F05 Query DSL | M04, M07 | 🟢 Baja (M04 es core) |
| F09 Block References | M04, M06, M08 | 🟡 Media |
| F16 Multi-graph | M02, M03, M04, M05 | 🟡 Alta |

---

## Heatmap de cambios

Para estimar riesgo al modificar un módulo:

| Módulo | Features afetadas | Risk Score |
|--------|-------------------|------------|
| M04 (frontend/db) | 15 | 🔴 HIGH |
| M01 (frontend/components) | 14 | 🔴 HIGH |
| M03 (frontend/handler) | 10 | 🟡 MEDIUM |
| M02 (frontend/state) | 8 | 🟡 MEDIUM |
| M06 (frontend/format) | 7 | 🟡 MEDIUM |
| M08 (graph-parser) | 5 | 🟡 MEDIUM |
| M09 (outliner) | 4 | 🟢 LOW |
| M05 (frontend/fs) | 3 | 🟢 LOW |
| M07 (frontend/search) | 3 | 🟢 LOW |

---

## Path de impacto para cambios

### Cambio en M04 (DataScript Schema)

```
M04 (schema)
    ↓ ALTER
F08 Tags, F09 Refs, F11 Timestamps, F12 Tasks
    ↓ IMPACT
M01 (components) — UI que renderiza estos campos
M03 (handlers) — Validación y transactions
M08 (parser) — Extracción de estos campos
```

### Cambio en M09 (Outliner)

```
M09 (outliner/tree)
    ↓ ALTER
F02 Outliner Tree (tree operations)
    ↓ IMPACT
M04 (transact) — Transaction handling
M03 (handlers) — Editor handlers
M01 (editor component) — User interaction
```

---

## Confianza del análisis

| Área | Confianza | Notas |
|------|-----------|-------|
| M01, M02, M03, M04 | 🟢 Alta | Módulos core, completos |
| M05, M06, M07 | 🟢 Alta | Protocolos bien definidos |
| M08, M09 | 🟢 Alta | Código externo completo |
| M10 (electron) | 🟡 Media | Wrapper, no analizado deep |
| M11 (mobile) | 🟡 Media | Wrapper, no analizado deep |
| M12 (plugins) | 🟡 Media | API incompleta en docs |
| M13 (publishing) | 🟡 Media | Código no analizado |

---

*Generado por Reversa Architect - 2026-05-02*
