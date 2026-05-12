# Roadmap detallado para cerrar gaps de cobertura

> Este roadmap prioriza el trabajo necesario para alinear
> `docs/reversa/`, la implementación Rust y la experiencia end-to-end.

## Objetivo

Cerrar la brecha entre:

- **visión documentada**
- **backend Rust realmente implementado**
- **producto usable end-to-end**

## Principios de priorización

1. Primero corregir lo que bloquea decisiones o genera confusión.
2. Después cerrar gaps de producto sobre backend ya existente.
3. Por último, implementar features nuevas o visionarias.

---

## Fase 0 — Reconciliación documental

### Objetivo

Establecer una base documental fiable antes de seguir ampliando producto.

### Entregables

#### 0.1 Fuente canónica del schema

- Unificar `docs/reversa/erd.md` y `docs/reversa/data-dictionary.md`
- Decidir una de estas opciones:
  - `erd.md` como vista visual y `data-dictionary.md` como source of truth
  - o un único documento principal y otro derivado automáticamente

**Resultado esperado**

- un listado único de entidades/campos/semántica
- naming consistente
- timestamps y tipos descritos sin ambigüedad

#### 0.2 Normalización de task markers y properties

- fijar forma canónica de task markers
- documentar equivalencias entre:
  - representación de dominio
  - representación persistida
  - representación en DSL
  - representación Rust
- consolidar reglas de normalización de properties

#### 0.3 Cerrar estrategia real de sync

- decidir si Quilt documenta oficialmente:
  - CRDT completo,
  - LWW con variantes,
  - o transición hacia una estrategia futura

**Resultado esperado**

- documentación honesta y alineada con código

### Prioridad

**Muy alta**

### Impacto

- reduce ambigüedad técnica
- mejora onboarding
- evita decisiones basadas en documentación desfasada

---

## Fase 1 — Matriz de impacto y contratos formales

### Objetivo

Hacer más segura la evolución del sistema.

### Entregables

#### 1.1 Crear `spec-impact-matrix.md`

Debe mapear:

- bounded context / crate
- entidades afectadas
- features dependientes
- impacto UI
- impacto MCP
- impacto sync
- impacto cognitive

#### 1.2 Formalizar Query DSL como contrato

- reconciliar `query-dsl-spec.md` con implementación real
- marcar explícitamente:
  - operadores soportados
  - edge cases
  - features pendientes

#### 1.3 Documentar event loop / hooks

- contratos de payload
- eventos emitidos
- side effects esperados
- retry semantics si aplica

### Prioridad

**Alta**

### Impacto

- facilita refactors seguros
- baja riesgo de regresión
- aclara extensibilidad del sistema

---

## Fase 2 — Cerrar gaps de producto sobre backend existente

### Objetivo

Aprovechar que buena parte del backend ya existe y convertirlo en experiencia
usable visible.

### Línea estratégica

El mayor retorno no está en crear nuevos motores backend, sino en **exponer bien
los que ya existen**.

### Entregables prioritarios

#### 2.1 Morning Briefing end-to-end

**Estado actual**

- backend existe en `quilt-cognitive`
- wiring parcial en UI

**Trabajo pendiente**

- UI estable para briefing
- estados de carga/error/vacío
- render de cognitive pulse
- render de decay alerts
- render de serendipity highlights

**Resultado esperado**

- primera feature cognitiva visible y usable

#### 2.2 Cognitive Dashboard / Graph View

**Estado actual**

- capacidades backend existen
- visualización UI no parece cerrada

**Trabajo pendiente**

- vista de clusters, frontiers y gaps
- navegación por nodos/páginas
- integración con queries o selección contextual

#### 2.3 Serendipity UI

**Estado actual**

- motor backend existe
- UI de notificaciones o descubrimientos no está cerrada

**Trabajo pendiente**

- feed de conexiones sugeridas
- acciones sobre sugerencias
- explicación mínima de por qué se propone cada enlace

#### 2.4 Query UI avanzada

**Estado actual**

- existe UI básica

**Trabajo pendiente**

- mejorar builder UX
- feedback de errores del DSL
- templates/queries guardadas si aplica
- mejores resultados y filtros interactivos

### Prioridad

**Alta**

### Impacto

- convierte backend ya hecho en valor real para usuario
- valida la propuesta AI-first de Quilt

---

## Fase 3 — Workflows de agentes y UI cognitiva avanzada

### Objetivo

Cerrar la brecha entre la visión de `quilt-ui-workflows.md` y el producto real.

### Entregables

#### 3.1 Agent Room

Implementar la experiencia de debate o colaboración multi-agente propuesta.

Debe aclararse primero:

- qué parte es visualización
- qué parte es orquestación MCP
- qué parte es persistencia de discusiones

#### 3.2 Focus mode con AI panel

Llevar al editor un flujo de copiloto cognitivo real.

Debe incluir:

- contexto de página/bloque activo
- sugerencias accionables
- consultas al backend cognitivo

#### 3.3 Decay monitor y weekly review

Exponer features de mantenimiento cognitivo como workflows de usuario.

### Prioridad

**Media-alta**

### Impacto

- acerca Quilt a su propuesta diferencial
- convierte capacidades internas en workflows reconocibles

---

## Fase 4 — Cerrar incertidumbres de sync y platform

### Objetivo

Madurar capacidades que son importantes, pero no el primer multiplicador de
valor visible.

### Entregables

#### 4.1 Sync contract final

- documentar semántica real
- validar escenarios conflictivos
- decidir roadmap de evolución si la ambición documental supera la realidad

#### 4.2 E2EE

Solo después de cerrar bien el contrato de sync y storage.

#### 4.3 File watching end-to-end

- cerrar wiring con sync/indexado/eventos

#### 4.4 WASM/browser como target de producto

- no solo compilación, también experiencia usable y soportada

### Prioridad

**Media**

---

## Priorización resumida por impacto

## Prioridad P0

- unificar schema docs
- aclarar estrategia de sync
- normalizar task markers / properties
- crear spec-impact-matrix

## Prioridad P1

- morning briefing end-to-end
- cognitive dashboard / graph view
- serendipity UI
- query UI avanzada

## Prioridad P2

- agent room
- focus mode con AI
- decay monitor
- weekly review

## Prioridad P3

- E2EE
- file watching totalmente integrado
- browser/WASM como producto completo

---

## Métrica de éxito propuesta

### Éxito documental

- existe una fuente canónica de schema
- no hay contradicciones abiertas entre docs principales
- cada feature propuesta está marcada como:
  - implementada
  - parcial
  - planificada

### Éxito técnico

- Morning Briefing usable desde UI
- al menos una vista cognitiva visible y estable
- Query UI alineada con capacidades reales del DSL
- sync documentado según comportamiento real

### Éxito de producto

- un usuario puede percibir claramente el valor AI-first sin depender de leer la
  documentación interna

---

## Recomendación final

El mejor orden de ejecución no es “más backend”, sino:

1. **reconciliar documentación crítica**
2. **exponer bien en UI lo que ya existe en backend**
3. **cerrar workflows cognitivos visibles**
4. **madurar sync/platform donde todavía hay ambigüedad**

Ese orden maximiza claridad, reduce riesgo y convierte más rápido la inversión
ya hecha en Rust en valor real de producto.
