# ADR: Block/Pages Discovery Endpoints

## Status

Implemented (2026-06-09)

## Context

El Judgment Day identificó dos gaps de discoverabilidad en la API:

1. **AgentActivityFeed** hardcodeaba `['agent::claude', 'agent::gemini', 'agent::gpt', 'agent::quilt']` — cualquier nuevo agente (e.g., `agent::deepseek`) era invisible hasta cambiar código.
2. **SearchModal** cargaba TODAS las páginas client-side via `api.listPages()` y filtraba con `includes()` — O(n) por keystroke, payload de MB en cada apertura.

## Decision

### Endpoint 1: `GET /api/v1/blocks/authors`

Retorna agentes distintos que han escrito bloques, descubriendo nuevos agentes automáticamente.

```
GET /api/v1/blocks/authors
Authorization: Bearer <key>

Response 200:
{ "authors": ["agent::claude", "agent::deepseek", "agent::gpt"] }
```

SQL: `SELECT DISTINCT created_by FROM blocks WHERE created_by LIKE 'agent::%'`

**Implementación**:
- `quilt-domain`: `BlockRepository::list_distinct_authors` trait
- `quilt-infrastructure`: `SqliteBlockRepository::list_distinct_authors` via `json_extract` sobre properties BLOB
- `quilt-server`: handler `list_distinct_authors` + route
- `quilt-ui`: `api.getDistinctAuthors()` → `AgentActivityFeed` usa la lista dinámica

### Endpoint 2: `GET /api/v1/pages/search?q=&limit=`

Busca páginas por nombre o título server-side, con límite para evitar payload excesivo.

```
GET /api/v1/pages/search?q=project&limit=20
Authorization: Bearer <key>

Response 200:
{ "pages": [{ "name": "project-alpha", "title": "Project Alpha" }, ...] }
```

SQL: `SELECT name, title FROM pages WHERE lower(name) LIKE '%project%' OR lower(IFNULL(title, '')) LIKE '%project%' LIMIT ?`

**Implementación**:
- `quilt-infrastructure`: `SqlitePageRepository::search_by_name_or_title`
- `quilt-server`: handler `search_pages` + route `/search` (registrado ANTES de `/:name` para evitar colisión)
- `quilt-ui`: `api.searchPages(query, limit?)` → `SearchModal` usa en vez de `listPages() + includes()`

## Considered Options

### Para AgentActivityFeed:
1. **Hardcoded list** (rechazado — requiere code change por cada nuevo agente)
2. **Polling agents via MCP** (overkill — agents no son entidades)
3. **`GET /api/v1/blocks/authors`** (aceptado — query directa, bajo costo, descubrible)

### Para SearchModal pages:
1. **Client-side filter de todas las páginas** (rechazado — O(n), MB de payload)
2. **Search engine dedicado** (overkill — LIKE es suficiente para <10k páginas)
3. **`GET /api/v1/pages/search`** (aceptado — server-side, bound, eficiente)

## Consequences

- `AgentActivityFeed` now auto-discovers new agents writing blocks
- `SearchModal` page search is O(1) por keystroke, bounded payload
- Dos nuevos endpoints en la API pública (rutas protegidas por auth)
- El SQL de `pages/search` hace `lower()` en cada fila — acceptable para <10k páginas

## References

- S2-02, S2-03 del Judgment Day (2026-06-09)
- `crates/quilt-server/src/handlers/blocks.rs`
- `crates/quilt-server/src/handlers/pages.rs`
