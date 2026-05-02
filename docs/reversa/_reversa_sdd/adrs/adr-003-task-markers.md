# ADR-003: Task Markers System

> Architecture Decision Record - Logseq
> Generado por: reversa-detective
> Fecha: 2026-05-02

---

## Status

🟢 CONFIRMADO - Implementado y en producción

---

## Context

Quilt es un outliner PKM que necesita tracking de tareas. Los task markers permiten a usuarios gestionar el ciclo de vida de tareas.

**Requerimientos**:
- Estados claros para tareas (todo, in-progress, done)
- Queries para filtrar por estado
- Compatibilidad con Org-mode y Markdown

---

## Decision

Implementar **Task Markers como propiedades especiales** con valores cerrados (closed values):

| Marker | Keyword | Descripción |
|--------|---------|-------------|
| `NOW` | `:logseq.property/status.doing` | En progreso |
| `LATER` | `:logseq.property/status.later` | Planificado |
| `TODO` | `:logseq.property/status.todo` | Por hacer |
| `DONE` | `:logseq.property/status.done` | Completado |
| `CANCELLED` | `:logseq.property/status.canceled` | Cancelado |

**Características**:
- Stored as `:logseq.property/status` enum
- Queries DSL: `(task todo done)` para filtrar
- Built-in queries predefinidas para cada estado
- Soporte para synonyms (`CANCELED` = `CANCELLED`)

---

## Evidence (Code)

**From `src/test/frontend/test/helper.cljs`**:
```clojure
{"TODO" :logseq.property/status.todo
 "DOING" :logseq.property/status.doing
 "DONE" :logseq.property/status.done
 "CANCELED" :logseq.property/status.canceled
 "CANCELLED" :logseq.property/status.canceled}
```

**From query DSL**:
```clojure
;; Query operators
:task  ;; Filter por estado de task
:priority ;; Filter por prioridad
```

---

## Consequences

**Positive**:
- ✅ Estados claramente definidos
- ✅ Queries poderosas para filtrar tareas
- ✅ Compatible con Org-mode
- ✅ Extensible para nuevos estados

**Negative**:
- ❌ Estados limitados a los predefinidos
- ❌ No hay transiciones automáticas (ej: TODO → DONE automático)

---

## Alternatives Considered

1. **Custom properties para status**: Users definen sus propios estados
   - ❌ Inconsistencia entre usuarios
   - ❌ Queries más complejas

2. **Tag-based tracking**: Usar tags como `#todo`, `#done`
   - ❌ No hay workflow estándar
   - ❌ Queries menos eficientes

3. **Closed enum (DECIDIDO)**: Estados predefinidos como enum
   - ✅ Consistencia
   - ✅ Queries simples
   - ⚠️ Menos flexible

---

*Documento generado automáticamente por Reversa Detective*
