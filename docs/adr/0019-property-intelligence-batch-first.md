# ADR-0019: Property Intelligence batch-first y properties como entidades de primera clase

Status: accepted

## Context

Quilt hoy tiene dos planos de properties:

1. **Properties persistidas** en `block.properties[]` como JSON blob
2. **Properties escritas en contenido** con sintaxis `key:: value`

Además, el sistema carece todavía de una **inteligencia de properties** global:

- no existe un registro canónico de qué properties existen
- no sabemos cuántas veces se usa cada una
- no medimos co-ocurrencias entre properties
- no hay prevención activa de duplicados (`due_date` vs `deadline`)
- los agentes MCP no tienen una herramienta única para descubrir, validar, sugerir y reutilizar properties

La investigación comparativa mostró cuatro patrones fuertes:

- **Tana**: los fields son entidades de primera clase y se sugieren/reutilizan
- **Datomic**: el esquema crece y nunca se rompe; se depreca o aliasa, no se borra
- **SHACL / RDF / Schema.org**: las properties tienen dominio, rango, jerarquía y metadatos propios
- **Wikidata / SchemaTree / Schema.org stats**: el uso y la co-ocurrencia de properties se pueden analizar y publicar

## Decision

**Quilt evoluciona las properties hacia un sistema batch-first de metadata inteligente.**

### 1. Las properties pasan a tener definición canónica

Se introduce un **Property Registry** global del workspace/grafo.

Cada property definida en Quilt tendrá:

- `key` canónica (snake_case, lowercase, sin espacios, `<= 32` chars)
- `display_label` humana
- `type` (text, number, date, select, boolean, url, reference)
- `status` (`active`, `deprecated`, `merged`, `alias`)
- metadatos de uso (`block_count`, `page_count`, `co_occurrence`, `first_seen_at`, `last_seen_at`)
- constraints y hints de UI

### 2. La key técnica se restringe y se normaliza

Las keys canónicas deben ser:

- lowercase
- sin espacios
- regex: `[a-z0-9_]+`
- longitud recomendada: `<= 32`

La UI puede aceptar input humano flexible pero Quilt normaliza:

- `Review Status` → `review_status`
- `due date` → `due_date`

La **label visible** queda desacoplada de la key técnica.

### 3. El descubrimiento de properties es batch-first

Ni la UI ni los agentes deben hacer una llamada por property.

Se define un endpoint REST y una tool MCP unificadas:

- `POST /api/v1/properties/batch`
- `quilt_properties_batch`

Una sola llamada puede mezclar:

- validación
- sugerencias
- stats
- co-ocurrencias
- recomendación contextual

### 4. Las properties tienen ciclo de vida, no solo valor

Una property puede pasar por:

- creación
- adopción creciente
- promoción a template/schema
- alias
- deprecación
- merge hacia otra key

El principio es **grow, never break**:

- no borrar keys viejas destructivamente
- deprecarlas y apuntarlas a un reemplazo
- mantener alias para backward compatibility

### 5. Las properties son queryables como metadata del grafo

Quilt debe poder responder preguntas como:

- qué properties existen
- cuáles son las más usadas
- cuáles están decayendo
- cuáles suelen aparecer juntas
- qué páginas/bloques usan una property
- qué property existente es más cercana a un concepto nuevo

## Architecture (DDD — sin acoplamiento a persistencia)

### Domain (`quilt-domain`)

- `PropertyDefinition` — value object: key, label, type, status, constraints
- `PropertyType` — enum: Text, Number, Date, Select, Boolean, Url, Reference
- `PropertyStatus` — enum: Active, Deprecated, Merged, Alias
- `PropertyRepository` — trait: `get_definition()`, `find_by_key()`, `search()`, `batch_stats()`, `save_definition()`, `list_all()`

### Application (`quilt-application`)

- `PropertyService` — orquesta registry, normalización, suggestions, batch operations
- DTOs: `PropertyBatchRequest`, `PropertyBatchResponse`, `PropertySuggestion`, `PropertyStats`

### Infrastructure (`quilt-infrastructure`)

- `SqlitePropertyRepository` — implementa `PropertyRepository` contra SQLite
- Tabla `property_definitions` + índices
- Generación VIRTUAL de SQLite para property indexing (ver DRAFT-property-indexing-strategy.md)

### Query (`quilt-query`)

- Extensiones al DSL: `(property-suggest ...)`, `(property-stats ...)`, `(property-cooccur ...)`
- `QueryCompiler` extension hooks para property-aware queries

### Presentation

- REST: `POST /api/v1/properties/batch`, `GET /api/v1/properties`
- MCP: `quilt_properties_batch` (`quilt_properties_batch_update` removed — grill session 2026-06-11: batch GET covers the common case; individual SET is more debuggable for agents)
- WASM: key normalization, fuzzy suggestion contra snapshot

## API Shape (batch-first)

### REST

```json
POST /api/v1/properties/batch

{
  "queries": [
    { "key": "status", "action": "validate" },
    { "key": "review status", "action": "suggest" },
    { "key": "priority", "action": "stats" },
    { "key": "due_date", "action": "suggest", "context": ["status", "priority"] }
  ]
}
```

### MCP

```json
quilt_properties_batch({
  "queries": [
    { "key": "status", "action": "suggest" },
    { "key": "priority", "action": "stats" },
    { "key": "deadline", "action": "suggest" },
    { "key": "review status", "action": "suggest", "context": ["status", "priority"] }
  ]
})
```

## Consequences

### Positivas

- menos duplicados semánticos
- mejores queries DSL
- mejor UX para usuarios al escribir properties
- menos llamadas REST/MCP por operación compleja
- agentes más consistentes y más seguros al reutilizar metadata existente
- posibilidad de construir vistas analíticas del grafo a nivel property
- DDD puro: el dominio no conoce SQL ni SQLite

### Costes

- nueva infraestructura de registry e índices
- normalización y migración de naming inconsistente
- necesidad de definir políticas de alias/deprecación
- mayor superficie de producto y de testing

## Rejected options

1. **Mantener properties como strings libres sin registry** — simple, pero condena a drift semántico y duplicados
2. **Solo validación individual por property** — genera N llamadas, peor UX y peor eficiencia agentica
3. **Permitir keys humanas con espacios como canónicas** — empeora parsing, queries, reuse y tooling
4. **Borrar properties viejas en merges/deprecaciones** — rompe backward compatibility y oculta trazabilidad
5. **Acoplar domain a SQLite directamente** — viola DDD (ADR-0008 ya establece persistencia como detail de infraestructura)

## Open questions

1. ~~¿Usamos `snake_case` o `kebab-case` como forma canónica?~~ → Recomendación: `snake_case`
2. ¿El registry es por workspace, por grafo completo o por namespace/página también?
3. ¿La normalización humana → key técnica debe ser automática o sugerida?
4. ¿Qué partes del analytics se calculan on-demand y cuáles se materializan?

## References

- ADR-0008: Persistencia como detail de infraestructura
- ADR-0004: DSL compartido con superconjunto para MCP
- Tana fields / supertags
- Datomic schema growth
- SHACL property shapes
- Schema.org usage statistics
- docs/grill/dsl-query-property-intelligence-analysis.md — competitive analysis
- docs/grill/property-intelligence-roadmap.md — implementation roadmap
