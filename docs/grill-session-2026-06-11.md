# Quilt Housekeeping Grill Session — 2026-06-11

## Contexto

Sesión de análisis crítico (auto-grill) sobre los items pendientes del proyecto Quilt. Focus: **máxima utilidad real para el usuario**, no features que suenan bien pero no se usan.

---

## Veredictos por item

| Item | Veredicto | Effort | Prioridad | Status |
|------|-----------|--------|-----------|--------|
| CRDT (quilt-core) | **BORRAR** | 30min | 🔴 Inmediato | ✅ Done |
| 4 property tests broken | **FIX** | 2h | 🔴 Inmediato | ✅ Done |
| 14 E2E test.skip | **test.fixme + issue** | 1h | 🔴 Inmediato | ✅ Done |
| ADR-0019 batch_update | **No implementar** | 30min | 🟡 Esta semana | ✅ Done |
| outliner-professional-roadmap.md | **Warning histórico** | 30min | 🟡 Esta semana | ✅ Done |
| Annotations + Comments | **IMPLEMENTAR** | 2-3 días | 🟢 Próximo | ✅ Done |
| Intent Search V3a (heurístico) | **IMPLEMENTAR** | 3-4 días | 🟡 Después | ✅ Done |
| Block Shape Detector | **DIFERIR** | — | ⏸️ Post-kNN | ⏸️ |
| SUNNY Phase 2-5 | **DIFERIR** | — | ⏸️ Post-telemetría | ⏸️ |
| Weighted Graph | **DIFERIR** | — | ⏸️ Sin caso UI | ⏸️ |

---

## Decisiones detalladas

### 🔴 CRDT — BORRAR

**Razón**: Single-user + SQLite local. 624 líneas que nadie consume. YAGNI puro.

```
crates/quilt-core/src/sync/crdt.rs — ELIMINAR
Si algún día hay visión multi-device → git log lo recupera
```

**Acción**:
1. Delete `crates/quilt-core/src/sync/crdt.rs`
2. Si `crates/quilt-core/src/sync/mod.rs` queda vacío → eliminar módulo
3. Commit: `chore: remove unwired CRDT sync (YAGNI, single-user)`

---

### 🔴 4 property tests — FIX

| Test | Problema | Fix |
|------|----------|-----|
| `order_proptest` | `services::order_utils` no existe en `services/mod.rs` | Agregar export o usar path correcto |
| `journal_day_proptest` | `JournalDay: PartialOrd` no derivada | Agregar `derive(PartialOrd)` al tipo |
| `sanitize_proptest` | Symbol eliminado referencing removed symbol | Encontrar el symbol actual o borrar test |
| `parser_proptest` | Runtime panics en ciertos inputs | Error handling o fix del parser |

**Acción**: `cargo test -p quilt-query -- --nocapture` para reproducir cada panic, luego fix.

---

### 🔴 14 E2E test.skip — Convertir a test.fixme + issue

**Patrón detectado**:
- 5 en `inline.spec.ts`: "WASM inline parsing may not be loaded" → indica flaky WASM loading
- 5 en `markers.spec.ts`: "Backend not available" → indica backend no arranca en headless
- Resto: issues diversos

**Acción**:
```typescript
// ANTES (miente — el test corre y pasa silenciosamente)
test.skip('something', async () => { ... });

// DESPUÉS (honesto — documenta el problema)
test.fixme('something — @issue:https://github.com/.../issue/XXX', async () => { ... });
```

**No borrar**: estos tests documentan problemas reales que hay que resolver (WASM loading flaky, backend en headless).

---

### 🟡 ADR-0019 batch_update — No implementar, quitar promesa

**Razón**: El batch GET (`quilt_properties_batch`) cubre el caso común. Un agente que quiere actualizar 5 properties → 5 llamadas分开 es más debuggable que una llamada batch. La optimización no justifica la complejidad.

**Acción**: Editar `docs/adr/0019-properties-mcp-tools.md` — quitar `quilt_properties_batch_update` de "planned tools". Solo deja `quilt_properties_batch` (ya implementado).

---

### 🟡 outliner-professional-roadmap.md — Warning histórico

**Razón**: El doc tiene refs a "ADR-0007 (CodeMirror 6)" que ahora son TipTap. Pero reescribir 11K líneas borra valor histórico.

**Acción**: Agregar frontmatter YAML con warning:
```yaml
---
warning:: "Histórico 2024-2025. ADR-0007 actual = TipTap, no CodeMirror 6."
deprecated:: true
---
```

---

## Feature Proposals

### 🟢 Annotations + Comments (unificados)

**Concepto unificado**: Usar `type:: annotation` con property `scope:: inline | block`

| scope | Descripción | Ejemplo |
|-------|-------------|---------|
| `inline` | Marca fragmento dentro de un bloque | El agente subraya "esta oración específica" |
| `block` | Comenta sobre un bloque entero | El usuario dice "este bloque necesita más contexto" |

**Infraestructura compartida**:
- Entity `Annotation` ya existe en `quilt-domain/src/entities/annotation.rs`
- Ambos son bloques hijo con `parent_id`, `created_by`, `resolved`
- Misma API REST, mismo MCP tool, mismo panel UI

**UI**:
- `scope:: inline` → rendering con `target-offset` y `target-length` como highlighting amarillo
- `scope:: block` → badge "💬 N comments" en el bloque
- Panel lateral: lista de annotations + comments con filtros

**Effort**: 2-3 días

**Acción wire-up**:
1. REST: `POST/GET /api/v1/blocks/:id/annotations`
2. MCP tools: `quilt_create_annotation`, `quilt_list_annotations`, `quilt_resolve_annotation`
3. UI: TipTap inline decoration para inline scope; badge + drawer para block scope
4. Panel: sidebar con lista filtrable

---

### 🟡 Intent Search V3a (heurístico, sin LLM)

**Concepto**: Usuario escribe "mostrame tareas abiertas por proyecto" → Quilt detecta patterns y genera DSL → muestra Table/Kanban.

**Por qué post-Annotations**: Intent Search necesita contenido en el grafo para ser útil. Con 0 bloques, "tareas abiertas por proyecto" devuelve vacío.

**Patterns heurísticos** (sin LLM, solo regex/keyword):

| Input | DSL generado |
|-------|--------------|
| `/open tasks/` | `status:: ["todo", "in-progress"], type:: task` |
| `/by {property}/` | `group_by:: {property}` |
| `/tasks for {name}/` | `type:: task, {name}:: {value}` |
| `/done this week/` | `status:: done, created:: [this week]` |
| `/project {name}/` | `{project:: {name}}` |

**Effort**: 3-4 días

**Acción wire-up**:
1. DSL generator: `quilt-query/src/heuristic.rs` — parse natural → AST
2. REST endpoint: `POST /api/v1/search/intent` → devuelve query AST + explanation
3. UI: Autocomplete en search modal cuando detecta intent pattern
4. Fallback: mostrar DSL generado para que el usuario lo edite

---

## No hacer ahora (y por qué)

| Item | Razón |
|------|-------|
| **Block Shape Detector** | Sin k-NN + telemetry, un detector determinístico tiene más falsos positivos que aciertos. Usuario lo ignora. |
| **SUNNY Phase 2-5** | Telemetry sin usuarios = tabla vacía. k-NN sin telemetry = ruido. Phase 1 (traits + scorer) ya funciona. |
| **Weighted Graph** | Peso 0.0-1.0 sin UI concreta ("las conexiones más fuertes") = número abstracto. ADR-0001 roza análisis semántico. |

---

## Roadmap propuesto

### Batch inmediato — Esta semana (~5h) ✅ COMPLETADO

| # | Tarea | Effort | Status |
|---|-------|--------|--------|
| 1 | BORRAR CRDT (`quilt-core/src/sync/crdt.rs`) | 30min | ✅ |
| 2 | FIX 4 property tests | 2h | ✅ |
| 3 | E2E: `test.skip` → `test.fixme` + issue | 1h | ✅ |
| 4 | ADR-0019: quitar promesa `batch_update` | 30min | ✅ |
| 5 | `outliner-professional-roadmap.md`: warning | 30min | ✅ |

### Batch próximo — Semana que viene (2-3 días) ✅ COMPLETADO

| # | Tarea | Effort | Status |
|---|-------|--------|--------|
| 6 | Annotations + Comments unificados | 2-3 días | ✅ |

### Batch después — (3-4 días) ✅ COMPLETADO

| # | Tarea | Effort | Status |
|---|-------|--------|--------|
| 7 | Intent Search V3a heurístico | 3-4 días | ✅ |

### Nunca (revisar en 6 meses)

| # | Tarea | Razón |
|---|-------|-------|
| 8 | Block Shape Detector | Post-kNN |
| 9 | SUNNY Phase 2-5 | Post-telemetría |
| 10 | Weighted Graph | Sin caso UI concreto |

---

## Resumen ejecutivo

```
Inmediato (esta semana, ~5h):
├── BORRAR CRDT (30min)
├── FIX 4 property tests (2h)
├── E2E test.skip → test.fixme (1h)
├── ADR-0019 batch_update promise remove (30min)
└── outliner-professional-roadmap.md warning (30min)

Próximo (semana que viene, 2-3 días):
└── Annotations + Comments (scope:: inline | block)

Después (3-4 días):
└── Intent Search V3a heurístico

NO hacer ahora:
├── SUNNY Phase 2-5
├── Block Shape Detector
└── Weighted Graph
```
