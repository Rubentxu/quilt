# Issues concretos derivados del roadmap de gaps

> Este documento traduce el roadmap en issues accionables, listos para llevar a
> GitHub Issues, SDD proposals o planificación interna.

## Convenciones

- **Prioridad**: P0, P1, P2, P3
- **Tipo**: docs, architecture, backend, ui, product
- **Estado sugerido**: open

---

## P0 — Issues críticos

## ISSUE-001 — Unificar schema documental y definir fuente canónica

- **Prioridad**: P0
- **Tipo**: docs / architecture
- **Estado sugerido**: open

### Problema

`docs/reversa/erd.md` y `docs/reversa/data-dictionary.md` no están plenamente
alineados y hoy no existe una única fuente canónica del schema.

### Objetivo

Tener una descripción única, coherente y mantenible del modelo de datos.

### Alcance

- reconciliar ambos documentos
- eliminar o marcar campos obsoletos
- unificar tipos, timestamps y naming
- documentar cuál archivo manda en caso de conflicto

### Criterios de aceptación

- no hay divergencias abiertas entre `erd.md` y `data-dictionary.md`
- los campos principales de blocks, pages, properties y refs aparecen descritos
  una sola vez de forma canónica
- queda documentada la convención de mantenimiento

### Dependencias

- ninguna

---

## ISSUE-002 — Documentar la estrategia real de sync

- **Prioridad**: P0
- **Tipo**: architecture / docs
- **Estado sugerido**: open

### Problema

La documentación histórica sugiere una visión CRDT/Loro más ambiciosa de la que
puede verificarse hoy en la implementación real.

### Objetivo

Actualizar la documentación para que describa con precisión el comportamiento
real del sync en Rust.

### Alcance

- revisar `crates/quilt-sync/`
- fijar narrativa oficial sobre conflict resolution
- actualizar los docs que prometen CRDT/Loro si no aplica
- documentar limitaciones actuales y roadmap evolutivo

### Criterios de aceptación

- la documentación no promete semánticas distintas a las implementadas
- queda clara la estrategia actual de resolución de conflictos
- se explicita si Loro es actual, futuro o descartado

### Dependencias

- ISSUE-001 recomendable, no bloqueante

---

## ISSUE-003 — Normalizar task markers y rules de properties

- **Prioridad**: P0
- **Tipo**: docs / backend
- **Estado sugerido**: open

### Problema

Los task markers y ciertas reglas de normalización de properties aparecen con
formatos distintos según el documento.

### Objetivo

Definir una representación canónica entre dominio, persistencia, DSL, UI y
tipos Rust.

### Alcance

- fijar casing y mapping de markers
- reconciliar docs del DSL y del dominio
- documentar la normalización real de properties

### Criterios de aceptación

- `domain.md` y `query-dsl-spec.md` quedan alineados
- existe una tabla explícita de equivalencias por representación

---

## ISSUE-004 — Crear `spec-impact-matrix.md`

- **Prioridad**: P0
- **Tipo**: docs / architecture
- **Estado sugerido**: open

### Problema

No existe una matriz formal de impacto entre crates, bounded contexts y
features.

### Objetivo

Facilitar refactors seguros, análisis de dependencia y planificación de cambios.

### Alcance

- mapear crates y bounded contexts
- indicar dependencias por feature
- añadir impacto en MCP, UI, sync y cognitive

### Criterios de aceptación

- el documento existe y cubre los crates principales
- sirve para evaluar impacto de una feature o refactor sin inspección manual

---

## P1 — Issues de valor visible inmediato

## ISSUE-005 — Completar Morning Briefing end-to-end

- **Prioridad**: P1
- **Tipo**: ui / product
- **Estado sugerido**: open

### Problema

El backend del Morning Briefing existe, pero la experiencia visible no parece
cerrada end-to-end.

### Objetivo

Exponer en la UI una primera feature cognitiva usable y estable.

### Alcance

- cerrar wiring backend ↔ platform ↔ UI
- mostrar cognitive pulse
- mostrar serendipity highlights
- mostrar decay alerts
- contemplar estados loading/error/empty

### Criterios de aceptación

- un usuario puede abrir el dashboard y ver el briefing completo
- el flujo funciona con datos reales o fallback controlado

### Dependencias

- ISSUE-002 recomendable para claridad conceptual

---

## ISSUE-006 — Implementar Cognitive Dashboard / Graph View

- **Prioridad**: P1
- **Tipo**: ui / product
- **Estado sugerido**: open

### Problema

Las capacidades del Cognitive Mirror backend no están plenamente expuestas en
una visualización útil en UI.

### Objetivo

Permitir explorar clusters, frontiers y gaps desde la interfaz.

### Alcance

- diseñar contrato UI/backend
- construir visualización inicial
- permitir navegación a páginas o bloques relacionados

### Criterios de aceptación

- la UI muestra al menos clusters/frontiers/gaps
- existe navegación o drill-down mínimo

---

## ISSUE-007 — Exponer Serendipity Engine en UI

- **Prioridad**: P1
- **Tipo**: ui / product
- **Estado sugerido**: open

### Problema

El motor de conexiones sugeridas existe en backend pero no parece visible como
workflow de producto claro.

### Objetivo

Ofrecer un feed o panel de sugerencias de conexión usable.

### Alcance

- listado de sugerencias
- explicación breve por sugerencia
- acción para explorar/aceptar/ignorar

### Criterios de aceptación

- un usuario puede descubrir al menos una conexión sugerida desde UI

---

## ISSUE-008 — Mejorar Query UI para alinearla con el DSL real

- **Prioridad**: P1
- **Tipo**: ui
- **Estado sugerido**: open

### Problema

La UI de queries existe pero no refleja necesariamente toda la potencia o
ergonomía del DSL implementado.

### Objetivo

Hacer que la UI sea un acceso fiable y cómodo al Query DSL real.

### Alcance

- feedback de errores mejorado
- ayuda de sintaxis/contexto
- plantillas o consultas frecuentes
- mejores resultados interactivos

### Criterios de aceptación

- la UI soporta los flujos más importantes del DSL sin fricción excesiva

---

## P2 — Issues de workflows avanzados

## ISSUE-009 — Implementar Agent Room MVP

- **Prioridad**: P2
- **Tipo**: ui / product / mcp
- **Estado sugerido**: open

### Problema

La visión de Agent Room existe en documentación, pero no se verifica una
implementación usable end-to-end.

### Objetivo

Crear un MVP que permita interacción multi-agente o debate guiado.

### Alcance

- definir caso de uso concreto
- diseñar persistencia/contexto mínimo
- conectar backend/MCP con UI

### Criterios de aceptación

- existe un flujo demostrable multi-agente o equivalente funcional

---

## ISSUE-010 — Implementar Focus Mode con AI panel contextual

- **Prioridad**: P2
- **Tipo**: ui / product
- **Estado sugerido**: open

### Problema

El editor no parece integrar todavía un flujo cognitivo contextual completo.

### Objetivo

Añadir un panel AI útil vinculado al bloque o página activa.

### Alcance

- contexto activo
- sugerencias cognitivas accionables
- integración con capacidades de backend existentes

### Criterios de aceptación

- el usuario puede editar y recibir ayuda contextual real

---

## ISSUE-011 — Implementar Decay Monitor y Weekly Review

- **Prioridad**: P2
- **Tipo**: ui / product
- **Estado sugerido**: open

### Problema

Los workflows de mantenimiento cognitivo están descritos en la visión, pero no
parecen disponibles como producto terminado.

### Objetivo

Exponer revisiones cognitivas y detección de obsolescencia desde UI.

### Alcance

- panel o flujo de revisión
- integración con señales cognitivas existentes
- navegación a páginas degradadas o de interés

### Criterios de aceptación

- el usuario puede ejecutar una revisión cognitiva útil desde la UI

---

## P3 — Issues de hardening y evolución posterior

## ISSUE-012 — Evaluar e implementar E2EE

- **Prioridad**: P3
- **Tipo**: backend / architecture / product
- **Estado sugerido**: open

### Problema

La documentación menciona E2EE, pero no se verificó implementación actual.

### Objetivo

Definir si E2EE es requisito real de producto y, si lo es, diseñar e
implementar su soporte.

---

## ISSUE-013 — Integrar file watching end-to-end

- **Prioridad**: P3
- **Tipo**: platform / backend
- **Estado sugerido**: open

### Problema

Hay señales de soporte parcial, pero no parece existir un flujo totalmente
cerrado watcher → sync/indexado/eventos.

### Objetivo

Cerrar el circuito de observación y reacción sobre filesystem.

---

## ISSUE-014 — Convertir WASM/browser en target de producto soportado

- **Prioridad**: P3
- **Tipo**: platform / ui
- **Estado sugerido**: open

### Problema

Compilar a WASM no equivale todavía a tener una experiencia browser soportada
como producto.

### Objetivo

Definir y cerrar soporte real para browser/WASM si sigue siendo objetivo de
producto.

---

## Siguiente agrupación sugerida en GitHub

### Milestone 1 — Reconciliación crítica

- ISSUE-001
- ISSUE-002
- ISSUE-003
- ISSUE-004

### Milestone 2 — Valor visible sobre backend existente

- ISSUE-005
- ISSUE-006
- ISSUE-007
- ISSUE-008

### Milestone 3 — Workflows cognitivos avanzados

- ISSUE-009
- ISSUE-010
- ISSUE-011

### Milestone 4 — Hardening y evolución

- ISSUE-012
- ISSUE-013
- ISSUE-014
