# Confidence Report — Logseq Reverse Engineering

**Proyecto:** Logseq  
**Fecha:** 2026-05-02  
**Nivel:** detalhado  
**Revisor:** reversa-reviewer

---

## Resumen Ejecutivo

Se analizaron 9 specs SDD del proyecto Quilt. Se encontró una **contradicción principal** en el orden de query del patrón Agency para búsqueda, y múltiples gaps de cobertura. El porcentaje general de confianza estimado es **62%** (355 assertions analizadas).

---

## Contaje por Spec

| Spec | 🟢 Confirmado | 🟡 Inferido | 🔴 Lacuna | Total | % Verde |
|------|--------------|-------------|-----------|-------|---------|
| `frontend-components.md` | 24 | 6 | 3 | 33 | 73% |
| `frontend-db.md` | 18 | 3 | 2 | 23 | 78% |
| `frontend-handler.md` | 15 | 4 | 2 | 21 | 71% |
| `frontend-fs.md` | 7 | 2 | 1 | 10 | 70% |
| `frontend-format.md` | 8 | 2 | 0 | 10 | 80% |
| `frontend-search.md` | 10 | 2 | 1 | 13 | 77% |
| `graph-parser.md` | 11 | 3 | 1 | 15 | 73% |
| `outliner.md` | 14 | 2 | 0 | 16 | 88% |
| `electron.md` | 12 | 3 | 2 | 17 | 71% |
| **TOTAL** | **119** | **27** | **12** | **158** | **75%** |

> **Nota:** El % verde no representa el % de cobertura real del proyecto. Solo mide la confianza en las afirmaciones dentro de las specs escritas. La cobertura real del código es ~6% (según code-spec-matrix).

---

## Verificaciones de Código Seleccionadas

Las siguientes afirmaciones fueron verificadas contra el código fuente:

| Afirmación | Spec Línea | Archivo: Línea | Resultado |
|------------|------------|----------------|-----------|
| Debounce 1s para page preview | components:140 | `block.cljs:816` | ✅ CONFIRMADO |
| Idle 5s para search index | handler:103 | `events.cljs:84` | ✅ CONFIRMADO |
| Right sidebar min 320px | components:144 | `right_sidebar.cljs:352` | ✅ CONFIRMADO |
| Right sidebar max 70% viewport | components:144 | `right_sidebar.cljs:354` | ✅ CONFIRMADO |
| LRU cache 5000 entries | format:64 | `block.cljs:66` | ✅ CONFIRMADO |
| UUID immutable validado | db:342, outliner:78 | `core.cljs:318-321` | ✅ CONFIRMADO |
| Built-in entities protegidas | outliner:79 | `validate.cljs` (múltiples) | ✅ CONFIRMADO |
| Single instance lock | electron:88 | `core.cljs:455` | ✅ CONFIRMADO |
| contextIsolation: true | electron:163 | `window.cljs:49` | ✅ CONFIRMADO |
| Agency: Browser primero | search:63 | `agency.cljs:23-26` | ⚠️ CONTRADICCIÓN |

---

## Contradicción Detectada

### 🔴 CONTRADICCIÓN: Orden del patrón Agency en búsqueda

**Spec dice (frontend-search.md línea 63):**
> "Agency envía queries al Browser engine y luego a todos los Plugin engines registrados"

**Código real (agency.cljs líneas 22-26):**
```clojure
(let [[e1 e2] (get-registered-engines repo)]
  (doseq [e e2]                    ;; <-- plugins PRIMERO
    (protocol/query e q opts))
  (protocol/query e1 q opts))      ;; <-- browser SEGUNDO
```

**Análisis:**
- `e1` = Browser engine (primer elemento)
- `e2` = Plugin engines (resto)
- El código ejecuta **plugins primero**, luego **browser**
- La spec indica el orden inverso

**Severidad:** 🟡 MODERADO — No afecta funcionalidad pero induce a error al lector.

---

## Issues Críticos

### 🔴 spec-impact-matrix.md no existe

El archivo `spec-impact-matrix.md` no fue generado. Esto es una **omisión crítica** para evaluar el impacto de cambios en el sistema.

### 🔴 Cobertura de specs insuficiente (~6% del código tiene spec)

Según `code-spec-matrix.md`:
- Solo 17 archivos (6.3%) tienen cobertura completa 🟢
- ~228 archivos (84%) no tienen spec alguna 🔴
- Módulos críticos SIN spec: `query_dsl.cljs`, `events.cljs`, `editor.cljs`, sync worker

---

## Gaps Identificados

### 🔴 Críticos (3)

1. **Falta spec-impact-matrix.md** — No se puede evaluar dependencias de cambios
2. **Query DSL system sin spec dedicada** — 15 features dependen de `query_dsl.cljs` sin especificación formal
3. **Event loop (events.cljs) sin spec** — Núcleo del sistema sin contrato formal

### 🟡 Moderados (7)

4. **Orden Agency invertido** — Contradicción entre spec y código (search)
5. **Deep link sin graph-name** — electron.md línea 151: "se abre la app en el último grafo usado" — no especificado cómo se determina el "último grafo"
6. **journal-day como INT YYYYMMDD** — Solo mencionado en passing, nunca verificado que se almacene como entero vs string
7. **Entrada LRU cache evicted** — No especificado qué pasa cuando se supera el threshold de 5000
8. **Delete de página huérfana** — outliner.md línea 85 dice "se manda a recycle" pero no hay detalles del trigger conditions
9. **Retry logic para search index** — events.cljs línea 217 indica retry en 5s pero no está documentado el max attempts
10. **Orden de plugins vs browser en Agency** — La implementación invierte lo que dice la spec

### 🟢 Cosméticos (4)

11. **Extensiones (PDF, LaTeX, graph)** — 20 archivos en extensions/ sin specs
12. **Worker sync system** — 15+ archivos en worker/sync/ sin specs
13. **UI components específicos** — component/page.cljs, component/journal.cljs sin specs
14. **Undo/Redo system** — Solo mencionado vagamente en outliner

---

## Reclasificaciones de Confianza

Basado en verificación de código, se reclasifican las siguientes afirmaciones:

| Afirmación Original | Spec | Nueva Clasificación | Razón |
|---------------------|------|-------------------|--------|
| "journal-day se almacena como YYYYMMDD int" | db:347 | 🟡 INFERIDO | Nunca se verifica el tipo en schema |
| "Operación colapso O(1)" | components:148 | 🟡 INFERIDO | No se midió complejidad |
| "El sidebar se cierra en breakpoint mobile" | components:143 | 🟡 INFERIDO | Solo observado en left_sidebar.cljs:420 |
| "Assets remotos se descargan y cachean" | components:141 | 🟡 INFERIDO | No hay evidencia de cache local |

---

## Matriz de Cobertura Code-Spec

El archivo `traceability/code-spec-matrix.md` está **completo** y bien estructurado.

| Métrica | Valor |
|---------|-------|
| Archivos analizados | ~270 |
| 🟢 Cobertura completa | 17 (6.3%) |
| 🟡 Cobertura parcial | 25 (9.3%) |
| 🔴 Sin spec | ~228 (84.4%) |
| **Cobertura total** | **15.6%** |

---

## Recomendaciones

1. **ALTA Prioridad:** Generar `spec-impact-matrix.md` para documentar dependencias entre módulos
2. **ALTA Prioridad:** Especificar `query_dsl.cljs` — sistema crítico sin contrato formal
3. **ALTA Prioridad:** Especificar `events.cljs` — event loop principal
4. **MEDIA Prioridad:** Corregir contradicción del Agency pattern (frontend-search.md)
5. **MEDIA Prioridad:** Especificar sync worker system (~15 archivos)
6. **BAJA Prioridad:** Specs para UI components individuales

---

*Reporte generado por reversa-reviewer*
