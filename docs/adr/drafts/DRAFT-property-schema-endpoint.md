# ADR-DRAFT: Endpoint de property keys con paginación y ADR de schema

Status: draft

## Context

TablePage y KanbanPage (`quilt-ui/src/pages/`) llaman `api.getBlockProperties('')` con string vacío para obtener todas las property keys del workspace. El backend (`crates/quilt-server/src/routes.rs`) solo expone propiedades por bloque específico, no un endpoint global de keys.

Esto rompe Table/Kanban — no pueden descubrir qué columnas mostrar. La sesión de auto-grill Q006-P1 (2026-06-07) decidió backend-first con paginación y ADR ligero de schema.

## Decision

**Implementar `GET /api/v1/properties/keys` con cursor pagination como endpoint backend. Frontend usa keys hardcodeadas como T0 unblocker en paralelo. ADR ligero para decisión de schema (normalizado vs denormalizado).**

### Endpoint

```
GET /api/v1/properties/keys?cursor=<last_key>&limit=<n>
```

Response:
```json
{
  "keys": ["status", "priority", "due-date", "project", "agent"],
  "next_cursor": "project",
  "has_more": true
}
```

### Frontend T0 unblocker

Mientras se implementa el endpoint, el frontend usa keys hardcodeadas conocidas:
```typescript
const KNOWN_PROPERTY_KEYS = [
  'status', 'priority', 'due-date', 'project', 'agent',
  'type', 'template', 'card-shape', 'created_by', 'tags'
];
```

### ADR de schema (pendiente)

Se requiere un ADR ligero que decida:
- **Normalizado**: tabla separada `properties` con FK a block, tipo enforced en DB
- **Denormalizado**: properties como JSON en columna del bloque (modelo actual de Logseq)

Trade-offs:
| | Normalizado | Denormalizado |
|---|---|---|
| Consistencia de tipos | Alta (DB enforced) | Baja (app-level) |
| Migración | Compleja | Trivial |
| Queries | JOINs, más lentas | FTS5-friendly, rápidas |
| WASM | Difícil (sin DB en browser) | Fácil (JSON puro) |
| Flexibilidad | Baja (schema rígido) | Alta (cualquier key) |

## Considered Options

1. **Solo frontend** (rechazado) — no escala, keys duplicadas en cada componente
2. **Backend sin paginación** (rechazado) — `SELECT DISTINCT key FROM properties` sin límite, riesgo de performance
3. **Backend con paginación + frontend T0 unblocker** — aceptado: backend correcto, frontend no bloqueado

## Consequences

- TablePage y KanbanPage dejan de llamar `getBlockProperties('')` (roto)
- `getBlockProperties(blockUuid)` se mantiene para propiedades de un bloque específico
- El endpoint de keys soporta cursor pagination para workspaces con miles de properties
- El ADR de schema (normalizado vs denormalizado) se escribe antes de Phase 2 del property system
- `quilt_list_property_keys` MCP tool expone el mismo endpoint a agentes

## References

- Q006-P1 (auto-grill 2026-06-07)
- `quilt-ui/src/pages/TablePage.tsx` — llama `getBlockProperties('')`
- `quilt-ui/src/pages/KanbanPage.tsx` — ídem
- `crates/quilt-server/src/routes.rs` — rutas montadas (sin /properties/keys)
