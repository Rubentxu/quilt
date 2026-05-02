# Gaps Report — Logseq Reverse Engineering

**Proyecto:** Logseq  
**Fecha:** 2026-05-02  
**Nivel:** detalhado

---

## Gaps Categorizados

### 🔴 Críticos (Bloqueantes)

#### GAP-001: spec-impact-matrix.md no existe
- **Severidad:** CRÍTICO
- **Categoría:** Missing Deliverable
- **Descripción:** El archivo `spec-impact-matrix.md` no fue generado. Este documento es esencial para entender dependencias entre módulos y evaluar el impacto de cambios.
- **Afecta:** Toda la documentación de la reverse engineering
- **Recomendación:** Generar spec-impact-matrix.md basado en:
  - Dependencias explícitas listed en cada spec (`## Dependências`)
  - Call graph analysis del codebase
  - Flujos de datos entre módulos

#### GAP-002: Query DSL system sin especificación formal
- **Severidad:** CRÍTICO  
- **Categoría:** Missing Spec
- **Descripción:** `frontend/db/query_dsl.cljs` es el motor de queries principal del sistema. 15+ features dependen de él pero no tiene spec SDD dedicada. El spec `frontend-db.md` lo menciona superficialmente.
- **Afecta:** 
  - `frontend/db/query_dsl.cljs`
  - `frontend/db/query_custom.cljs`
  - `frontend/db/query_react.cljs`
- **Recomendación:** Crear `sdd/query-dsl.md` detallando:
  - Gramática completa de operators (and, or, not, between, task, priority, page, property)
  - Reglas de parsing recursivo
  - Transformación a Datalog
  - Time helpers (today, -7d, +1w)
  - Límites y edge cases

#### GAP-003: Event loop principal sin contrato formal
- **Severidad:** CRÍTICO
- **Categoría:** Missing Spec
- **Descripción:** `frontend/handler/events.cljs` es el núcleo de la aplicación. Implementa el event loop asíncrono que procesa todas las acciones del usuario. No tiene spec SDD.
- **Afecta:**
  - `src/main/frontend/handler/events.cljs`
  - Todos los `defmethod handle` asociados
- **Recomendación:** Crear `sdd/event-system.md` detallando:
  - Todos los eventos posibles (tabla completa de tipos)
  - Contratos de payload para cada evento
  - Orden de procesamiento
  - Error handling y Sentry integration
  - Condiciones de carrera conocidas

---

### 🟡 Moderados (No bloqueantes pero importantes)

#### GAP-004: Contradicción en orden del Agency pattern
- **Severidad:** MODERADO
- **Categoría:** Contradiction
- **Descripción:** `frontend-search.md` línea 63 dice que Agency envía queries "al Browser engine y luego a todos los Plugin engines". El código en `agency.cljs:23-26` hace el orden inverso (plugins primero, browser segundo).
- **Afecta:** `frontend/search/agency.cljs`
- **Recomendación:** Corregir la spec para reflejar el comportamiento real (plugins primero) o cambiar el código si el orden importa para la semántica.

#### GAP-005: Deep link sin graph-name tiene comportamiento indefinido
- **Severidad:** MODERADO
- **Categoría:** Incomplete Spec
- **Descripción:** `electron.md` línea 151 indica que si `logseq://` no especifica graph-name, "se abre la app en el último grafo usado o se muestra la pantalla de selección". No está especificado:
  - Cómo se determina el "último grafo"
  - Si se persiste entre sesiones
  - Qué pasa si no hay último grafo (primera ejecución)
- **Afecta:** `electron/core.cljs:setup-deeplink!`
- **Recomendación:** Especificar el algoritmo de fallback y verificar implementación.

#### GAP-006: Tipo de dato journal-day no verificado
- **Severidad:** MODERADO
- **Categoría:** Unverified Type
- **Descripción:** Múltiples specs mencionan que `journal-day` se almacena como entero YYYYMMDD, pero:
  - No hay verificación del schema de DataScript
  - No se confirma que sea `:db/type :db.type/int` vs `:db.type/string`
  - Puede causar bugs sutiles en comparaciones
- **Afecta:** `deps/db/src/logseq/db/schema.cljs`
- **Recomendación:** Verificar schema y corregir specs si es necesario.

#### GAP-007: Estrategia de eviction del LRU cache no especificada
- **Severidad:** MODERADO
- **Categoría:** Incomplete Spec
- **Descripción:** `frontend-format.md` indica que hay un LRU cache con 5000 entries, pero:
  - No especifica el algoritmo de eviction (LRU, LFU, FIFO)
  - No indica qué pasa cuando se supera el threshold
  - No hay especificación de TTL o cleanup
- **Afecta:** `frontend/format/block.cljs:66`
- **Recomendación:** Especificar estrategia de eviction y agregar a Requisitos Não Funcionais.

#### GAP-008: Condiciones para páginas huérfanas en recycle
- **Severidad:** MODERADO
- **Categoría:** Incomplete Spec
- **Descripción:** `outliner.md` línea 85 dice que páginas huérfanas "se mandan a recycle" pero no especifica:
  - Exactamente qué constituye una página huérfana
  - Si hay unflushed transactions que afectan esta decisión
  - El tiempo entre quedar huérfana y ser movida a recycle
- **Afecta:** `deps/outliner/src/logseq/outliner/core.cljs`
- **Recomendación:** Especificar trigger conditions con precisión.

#### GAP-009: Retry logic para search index build
- **Severidad:** MODERADO
- **Categoría:** Incomplete Spec
- **Descripción:** `handler/events.cljs` programa retry en 5s cuando falla el search index build, pero no está documentado:
  - Máximo número de reintentos
  - Qué pasa después del último retry fallido
  - Si hay backoff exponencial
- **Afecta:** `handler/events.cljs:schedule-search-index-build!`
- **Recomendación:** Documentar retry policy completo.

#### GAP-010: Worker sync system sin specs
- **Severidad:** MODERADO
- **Categoría:** Missing Spec
- **Descripción:** 15+ archivos en `worker/sync/` no tienen specs. Este sistema maneja:
  - Sincronización de cambios entre cliente y servidor
  - Presencia colaborativa (quien está editando qué)
  - End-to-end encryption
  - Conflict resolution
- **Afecta:** Todo el subsistema `worker/sync/`
- **Recomendación:** Priorizar `sync.cljs` y `auth.cljs` para crear specs.

---

### 🟢 Cosméticos (Mejoras menores)

#### GAP-011: UI components sin specs individuales
- **Severidad:** BAJO
- **Categoría:** Missing Spec
- **Descripción:** `component/page.cljs`, `component/journal.cljs`, `component/query.cljs` no tienen specs dedicadas. Es entendible por serem low-level UI, pero limita la capacidad de refactoring.
- **Recomendación:** Opcional, bajo prioridad.

#### GAP-012: Extensiones sin cobertura
- **Severidad:** BAJO
- **Categoría:** Missing Spec
- **Descripción:** 20 archivos en `extensions/` (PDF, LaTeX, graph, etc.) sin specs. Extensions son API pública para plugins.
- **Recomendación:** Priorizar si hay planes de formalizar la Plugin API.

#### GAP-013: Undo/Redo system vago
- **Severidad:** BAJO
- **Categoría:** Incomplete Spec
- **Descripción:** `undo_redo.cljs` solo mencionado vagamente. No hay especificación de:
  - Transaction history format
  - Conflict resolution en concurrent edits
  - Memory implications de mantener history
- **Recomendación:** Especificar si el sistema es crítico para el usuario.

#### GAP-014: Commands y shortcuts sin spec
- **Severidad:** BAJO
- **Categoría:** Missing Spec
- **Descripción:** `commands.cljs` y `modules/shortcut/` no tienen specs. El sistema de comandos es parte de la UX pública.
- **Recomendación:** Opcional, bajo prioridad.

---

## Stats Resumen

| Severidad | Count | % del Total |
|-----------|-------|-------------|
| 🔴 Críticos | 3 | 21% |
| 🟡 Moderados | 7 | 50% |
| 🟢 Cosméticos | 4 | 29% |
| **Total** | **14** | 100% |

---

## Recomendaciones de Prioridad

### Inmediato (1-2 semanas)
1. **GAP-001:** Generar spec-impact-matrix.md
2. **GAP-003:** Especificar event system (events.cljs)

### Corto plazo (1 mes)
3. **GAP-002:** Especificar Query DSL
4. **GAP-004:** Corregir contradicción Agency
5. **GAP-005:** Especificar deep link fallback

### Medio plazo (2-3 meses)
6. **GAP-006:** Verificar journal-day type
7. **GAP-008:** Especificar orphan page handling
8. **GAP-009:** Documentar retry logic
9. **GAP-010:** Especificar sync system

### Largo plazo (backlog)
10-14. Gaps cosméticos restantes

---

*Reporte generado por reversa-reviewer*
