# ADR-DRAFT: SavedView como rol de bloque `type:: view` componiendo referencia a Query

Status: draft

## Context

El documento de investigación propuso unificar Table, Kanban, Query, Dashboard bajo un modelo de "Saved View". La sesión de auto-grill (Q004-P1 + Q011-P2, 2026-06-07) rechazó dos propuestas:

1. Q004-P1: extender `type:: query` con `name::`, `icon::`, `pinned::` → property-bag anti-pattern, colisión con CONTEXT.md que ya separa `type:: view` de `type:: query`
2. Q011-P2: volvió a proponer property-bag sobre `type:: view` → repetición del anti-patrón ya rechazado

El fork fue resuelto por decisión del arquitecto: composición sobre herencia. Un bloque view referencia un bloque query.

## Decision

**SavedView es un bloque con `type:: view` (rol ya definido en CONTEXT.md) que compone una referencia a un bloque query vía `data-source::`. NO es una entidad de dominio separada.**

### Modelo de propiedades

| Property | Type | Required | Purpose |
|----------|------|----------|---------|
| `type::` | role | Sí | `view` |
| `view-type::` | select | Sí | `table` \| `kanban` \| `calendar` \| `list` \| `graph` \| `cards` \| `timeline` |
| `data-source::` | block-ref | Sí | UUID del bloque query |
| `view-name::` | string | No | Nombre legible |
| `view-icon::` | string | No | Icono Lucide |
| `view-pinned::` | checkbox | No | Pin al sidebar/command center |
| `group-by::` | property-key | No | Propiedad de agrupación |
| `sort::` | json | No | Configuración de orden |

### Composición, no herencia

```
Bloque A (type:: query)
├── dsl:: (and (task TODO) (project "quilt"))
└── ... otras properties ...

Bloque B (type:: view)
├── view-type:: kanban
├── data-source:: <uuid-del-bloque-A>
├── view-name:: Tareas de Quilt
└── group-by:: priority

Bloque C (type:: view)
├── view-type:: table
├── data-source:: <uuid-del-bloque-A>   ← MISMO query
├── view-name:: Lista de tareas
└── sort:: [{"field": "due-date", "dir":"asc"}]
```

Múltiples views pueden referenciar el mismo query. Misma data, múltiples renderers. El patrón ya existe en Quilt: `((block-ref))` es el mecanismo de referencias entre bloques.

### Por qué NO es una entidad separada

1. ADR-0007 ya rechazó entidades separadas para cards ("no creés una tabla cards cuando podés usar `template::` como propiedad de bloque")
2. Cada nueva entidad fragmenta el modelo de propiedades y requiere migraciones
3. Los views son consultables/descubribles vía la misma infraestructura de bloques
4. El sistema de roles de Quilt ES su type system — `type:: view` ya existe en CONTEXT.md

## Considered Options

1. **Entidad separada (tabla `saved_views`)** (rechazado) — migración SQLite, repositorio nuevo, viola "todo es un bloque"
2. **Extender `type:: query` con properties de view** (rechazado por Q004-P1) — property-bag, colisión CONTEXT.md
3. **Property-bag sobre `type:: view`** (rechazado por Q011-P2) — mismo anti-patrón
4. **Bloque `type:: view` con `data-source::`** — aceptado: composición, sin migración, alineado con CONTEXT.md

## Consequences

- CONTEXT.md ya define `view` como rol — se implementa, no se redefine
- `data-source::` usa el mecanismo `((block-ref))` existente
- El frontend renderiza según `view-type::`: TableView, KanbanView, CalendarView, etc.
- ViewContainer (page-level) consume el bloque view y delega al LayoutEngine correspondiente
- CardRenderer (block-level) no se modifica — opera a nivel de bloque individual

## References

- Q004-P1 y Q011-P2 (auto-grill 2026-06-07)
- CONTEXT.md: `view` como rol de bloque, `query` como rol separado
- ADR-0007: Template-driven block cards (rechaza entidades separadas)
- Resolución del arquitecto (2026-06-07): SavedView es rol de bloque
