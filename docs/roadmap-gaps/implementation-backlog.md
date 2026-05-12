# Backlog de implementación accionable

> Backlog derivado del análisis de cobertura de features y de los gaps
> documentales detectados en `docs/reversa/`.

## Criterios de prioridad

- **P0**: bloquea decisiones, genera ambigüedad o afecta múltiples áreas.
- **P1**: desbloquea valor visible para usuario sobre backend ya existente.
- **P2**: mejora workflows avanzados o cierra features de producto importantes.
- **P3**: hardening o evolución posterior.

## P0 — Reconciliación crítica

### P0.1 Unificar schema documental

**Objetivo**

Convertir `docs/reversa/erd.md` y `docs/reversa/data-dictionary.md` en una
descripción coherente del modelo de datos.

**Tareas**

- Comparar ambos documentos campo por campo.
- Elegir una fuente canónica.
- Eliminar o marcar campos obsoletos.
- Alinear tipos, timestamps y naming.
- Documentar qué archivo manda en caso de conflicto.

**Entregables**

- `docs/reversa/data-dictionary.md` reconciliado
- `docs/reversa/erd.md` alineado o reducido a vista derivada

**Impacto**

Muy alto. Reduce ambigüedad estructural en todo el proyecto.

---

### P0.2 Fijar estrategia real de sync

**Objetivo**

Resolver la diferencia entre la visión documental tipo CRDT/Loro y la semántica
real implementada en Rust.

**Tareas**

- Revisar `crates/quilt-sync/` como source of truth.
- Decidir narrativa oficial: LWW actual, CRDT híbrido o roadmap hacia otro
  modelo.
- Actualizar `docs/reversa/rust-reimplementation-proposal.md` y documentos
  relacionados.
- Documentar explícitamente qué garantías ofrece hoy el sync.

**Entregables**

- sección de sync corregida en docs
- nota de compatibilidad/limitaciones actuales

**Impacto**

Muy alto. Evita promesas falsas sobre conflictos y replicación.

---

### P0.3 Normalizar task markers y properties

**Objetivo**

Definir representaciones canónicas para markers y reglas de normalización.

**Tareas**

- Definir forma canónica de task markers.
- Mapear representación Rust, persistida, DSL y UI.
- Alinear reglas de normalización de properties.
- Documentar ejemplos válidos e inválidos.

**Entregables**

- actualización de `docs/reversa/domain.md`
- actualización de `docs/reversa/query-dsl-spec.md`

**Impacto**

Alto. Reduce bugs de parsing, consultas y serialización.

---

### P0.4 Crear `spec-impact-matrix.md`

**Objetivo**

Tener una matriz de impacto para refactors y evolución del sistema.

**Tareas**

- Mapear bounded contexts y crates.
- Identificar dependencias por feature.
- Añadir impacto en UI, MCP, sync y cognitive.
- Definir cómo actualizar la matriz cuando cambie una feature.

**Entregables**

- `docs/reversa/spec-impact-matrix.md`

**Impacto**

Muy alto para mantenimiento y planificación.

---

## P1 — Convertir backend existente en valor visible

### P1.1 Completar Morning Briefing end-to-end

**Objetivo**

Hacer visible y usable la capacidad ya implementada en backend.

**Tareas**

- Revisar `crates/quilt-cognitive/src/morning_briefing/`.
- Cerrar wiring Tauri/UI si falta.
- Completar vista de dashboard en `quilt-ui`.
- Mostrar cognitive pulse, decay alerts y serendipity highlights.
- Añadir estados de loading, vacío y error.

**Entregables**

- briefing usable desde UI
- tests básicos de render/flujo

**Impacto**

Muy alto. Primera feature cognitiva visible para usuario.

---

### P1.2 Completar Cognitive Dashboard / Graph View

**Objetivo**

Visualizar capacidades de Cognitive Mirror ya existentes.

**Tareas**

- Diseñar contrato de datos UI ↔ backend.
- Crear vista de clusters, frontiers y gaps.
- Permitir navegación desde métricas a páginas/bloques relevantes.
- Añadir filtros o contexto por página/área.

**Entregables**

- vista cognitiva navegable

**Impacto**

Alto. Convierte capacidades backend en insight visible.

---

### P1.3 Completar Serendipity UI

**Objetivo**

Exponer el motor de conexiones sugeridas al usuario.

**Tareas**

- Diseñar feed/listado de sugerencias.
- Mostrar explicación breve por conexión.
- Añadir acción para aceptar, explorar o descartar sugerencia.
- Definir persistencia o feedback loop si aplica.

**Entregables**

- feed funcional de descubrimientos

**Impacto**

Alto. Hace tangible la propuesta AI-first.

---

### P1.4 Elevar Query UI de básica a operativa

**Objetivo**

Alinear la UI de consultas con las capacidades reales del DSL.

**Tareas**

- Mejorar feedback de errores.
- Añadir sugerencias/autocompletado si procede.
- Exponer operadores importantes de forma guiada.
- Permitir reutilizar consultas frecuentes.

**Entregables**

- query UI más cercana al contrato de `query-dsl-spec.md`

**Impacto**

Alto. Incrementa usabilidad inmediata.

---

## P2 — Workflows avanzados de producto

### P2.1 Implementar Agent Room

**Objetivo**

Crear una experiencia coherente para workflows multi-agente.

**Tareas**

- Definir el caso de uso principal.
- Diseñar persistencia de intercambio o debate.
- Conectar MCP/backend cognitivo con UI.
- Resolver estados, contexto y trazabilidad mínima.

**Entregables**

- MVP de Agent Room

**Impacto**

Medio-alto. Feature diferencial, pero no base.

---

### P2.2 Implementar Focus Mode con AI panel

**Objetivo**

Incorporar ayuda cognitiva contextual en el editor.

**Tareas**

- Definir contexto mínimo del panel.
- Conectar bloque/página activa con capacidades cognitivas.
- Mostrar sugerencias relevantes y accionables.
- Evaluar coste cognitivo y evitar sobrecarga visual.

**Entregables**

- modo foco con panel contextual usable

**Impacto**

Medio-alto.

---

### P2.3 Implementar Decay Monitor y Weekly Review

**Objetivo**

Cerrar workflows de mantenimiento cognitivo prometidos por la visión del
producto.

**Tareas**

- Definir qué métricas se presentan.
- Reutilizar datos del backend cognitivo.
- Diseñar UI simple y accionable.
- Conectar con navegación o refactor del conocimiento.

**Entregables**

- pantallas o paneles de revisión cognitiva

**Impacto**

Medio.

---

## P3 — Hardening y evolución posterior

### P3.1 E2EE

**Objetivo**

Evaluar e implementar cifrado extremo a extremo solo después de cerrar el
contrato real de sync.

### P3.2 File watching totalmente integrado

**Objetivo**

Cerrar el ciclo file watcher → indexado/sync/eventos.

### P3.3 WASM/browser como producto soportado

**Objetivo**

Pasar de compilación exitosa a experiencia soportada y documentada.

---

## Dependencias recomendadas entre bloques

1. **P0.1 + P0.2 + P0.3** antes de nuevas promesas funcionales.
2. **P1.1** antes de Agent Room o Focus Mode.
3. **P1.2 y P1.3** antes de workflows cognitivos complejos.
4. **P2** una vez estabilizada la base documental y la UI cognitiva básica.
5. **P3** cuando el producto visible ya refleje la mayor parte del valor real.

---

## Orden sugerido de ejecución real

### Sprint A

- P0.1 Unificar schema documental
- P0.2 Cerrar estrategia de sync
- P0.3 Normalizar markers/properties

### Sprint B

- P0.4 Crear spec-impact-matrix
- P1.1 Completar Morning Briefing

### Sprint C

- P1.2 Cognitive Dashboard
- P1.3 Serendipity UI

### Sprint D

- P1.4 Query UI avanzada
- P2.3 Decay Monitor / Weekly Review

### Sprint E

- P2.1 Agent Room
- P2.2 Focus Mode con AI panel

### Sprint F+

- P3.1 E2EE
- P3.2 File watching integrado
- P3.3 WASM/browser soportado

---

## Criterio de cierre del backlog

Se podrá considerar que la brecha principal está cerrada cuando:

- la documentación crítica no se contradiga,
- las features backend más importantes tengan reflejo visible en UI,
- y las features “estrella” documentadas de Quilt no dependan de suposiciones
  sino de flujos verificables end-to-end.
