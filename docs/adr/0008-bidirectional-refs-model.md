# ADR-0008: Modelo de refs bidireccionales — RefIndex en dominio, persistencia como detalle de infraestructura

Status: accepted

Quilt mantiene un índice bidireccional de referencias (`RefIndex`) como value object del dominio, usando dos `HashMap<Uuid, HashSet<Ref>>` (forward y reverse) que proporcionan O(1) para lookup de backlinks. La persistencia (SQLite con tabla `refs`, FTS5 para unlinked references) es un detail de infraestructura detrás del trait `RefRepository`. Si mañana se cambia SQLite por otra tecnología, el dominio no se modifica.

## Context

Logseq DB usa DataScript con atributo `:block/refs` de cardinalidad `many`. DataScript indexa automáticamente el reverso via AVET index, dando O(log n) para backlinks. Quilt no usa DataScript — necesita su propio mecanismo de indexación bidireccional.

La arquitectura DDD de Quilt exige que el modelo de refs sea agnóstico de persistencia. El dominio define QUÉ es una ref y CÓMO se consulta; la infraestructura define DÓNDE se guarda.

## Decision

### Capas

**Domain (`quilt-domain`)**:
- `Ref` — value object: `target: Uuid`, `ref_type: RefType`
- `RefType` — enum: `PageRef`, `BlockRef`, `Tag`, `Alias`
- `RefIndex` — value object: dual `HashMap` (forward + reverse), métodos `add_ref()`, `remove_ref()`, `get_backlinks()`, `backlink_count()`. Pure Rust, sin dependencias externas, WASM-compatible.
- `RefRepository` — trait: `get_forward_refs()`, `get_backlinks()`, `sync_refs()`, `rebuild_index() -> RefIndex`

**Application (`quilt-application`)**:
- `RefService` — orquesta: parse refs → diff → sync repo → update index → emit event
- `BacklinkQuery` — query service: `get_linked_refs()`, `get_unlinked_refs()` (v2)

**Infrastructure (`quilt-infrastructure`)**:
- `SqliteRefRepository` — implementa `RefRepository`: tabla `refs`, índices, FTS5
- Migrations: schema SQL vive exclusivamente aquí

### Write path

1. Block content cambia → `InlineParser` extrae `Vec<Ref>` en el dominio
2. `RefService` diff old refs vs new refs
3. `RefRepository::sync_refs(block_id, new_refs)` — persiste diff
4. `RefIndex::add_ref()` / `RefIndex::remove_ref()` — actualiza in-memory
5. Emite `BacklinksChanged` event

### Read path

1. `RefIndex::get_backlinks(page_id)` — O(1) desde memoria
2. Si no está cargado: `RefRepository::rebuild_index()` al arrancar
3. Unlinked refs (v2): FTS5 via `RefRepository`

### Phases

- **v1**: Tabla `refs` + `RefIndex` in-memory. Parser escribe refs al guardar bloque. Backlinks O(1). Sin unlinked refs.
- **v2**: FTS5 para unlinked references. `page_stats` desnormalizado opcional.

## Considered Options

1. **RefIndex en dominio + RefRepository trait + SQLite como infra** — accepted: DDD puro, agnóstico de persistencia, testeable sin DB, WASM-compatible.
2. **Tabla `refs` con `target_type` discriminatorio (acoplado a SQL)** — rejected: viola DDD, el dominio conoce SQL.
3. **Dos tablas separadas (block_page_ref + block_block_ref)** — rejected: más complejo, viola DDD, no extensible.
4. **Extraer refs al vuelo desde contenido (sin índice)** — rejected: O(n) por consulta, inaceptable para grafos grandes.

## Consequences

- El dominio nunca ve SQL. `RefRepository` es un trait con métodos Rust.
- `RefIndex` es serializable y reconstruible desde cualquier fuente.
- El parser inline (`InlineParser`) es del dominio. Qué hacer con las refs extraídas es del servicio de aplicación.
- Las queries del DSL usan `RefIndex`, no SQL. `(page-ref "Rust")` consulta `RefIndex::get_backlinks()`.
- Testing: se puede testear el modelo completo de refs con un `MockRefRepository` sin tocar SQLite.
- `petgraph` se puede usar para algoritmos de grafo (componentes conexos, detección de ciclos, caminos) sobre datos derivados del `RefIndex`, sin depender de la capa de persistencia.
