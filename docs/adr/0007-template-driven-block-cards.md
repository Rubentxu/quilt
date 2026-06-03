# ADR-0007: Template-driven block cards via `template::` + `card-shape::`

Status: accepted

## Context

Quilt hereda de Logseq la convención de templates como páginas con prefijo `template/`, y `Page::is_template_name()` (crates/quilt-domain/src/entities/page.rs:160) las detecta por nombre. La clonación de templates para crear páginas nuevas ya existe vía `POST /api/v1/pages/from-template` (crates/quilt-server/src/handlers/pages.rs:425). Sin embargo:

1. **No hay templates a nivel de bloque**. La activación visual de cards en el outliner está hardcodeada en dos componentes React (`ReferenceCard`, `ContentCard` en `quilt-ui/src/shared/components/`) y se decide por un switch sobre la propiedad `type::` (quilt-ui/src/features/outliner-tiptap/PageView.tsx:33-39, `getBlockType`). No extensible.

2. **Los agentes AI no descubren templates**. CONTEXT.md define Template como "contrato entre agente y usuario" y ADR-0003 dice "los templates definen la interface entre agente y usuario: structure + types", pero el MCP server no expone ninguna tool ni resource para templates. Un agente tiene que listar todas las páginas y filtrar por prefijo.

3. **El usuario no puede estilizar sus cards**. ReferenceCard y ContentCard son componentes React fijos. Cambiar colores, iconos, o el wrapping visual requiere tocar código.

4. **CONTEXT.md y ADR-0006 ya prometen `template::` como propiedad de bloque de primera clase**, pero el código no lo implementa (quilt-core/src/schema/properties.rs no la registra).

## Decision

**Las cards del outliner se activan por la propiedad `template:: <name>` del bloque, donde `<name>` referencia una template page existente (prefijo `template/`). El shape visual de la card se determina por la propiedad `card-shape::` en la template page, no por código hardcodeado.**

### Componentes

1. **`template::` como propiedad de bloque tipada** (string, valor = nombre de template page). Se registra en `BUILTIN_PROPERTIES` con `PropertyDefinition`. Reemplaza a `type:: reference` / `type:: documentacion` como discriminador de card.

2. **`card-shape::` como propiedad de la template page** (no del bloque). Valores en V1: `reference`, `content`, `inline`. La template page declara su shape visual. Un `CardRenderer` genérico en el frontend interpreta este shape:
   - `reference` → card plana con tabla de metas + acciones (abrir, copiar)
   - `content` → card colapsable con header + contenido interno
   - `inline` → bloque normal sin card wrapper, solo decoración (icono, color, cssclass)

3. **`cssclass::` como propiedad de la template page**. Cuando un bloque activa esa template, el wrapper del bloque recibe esa(s) clase(s) CSS. El usuario define los estilos en un CSS snippet. Inspirado en Obsidian `cssclasses` (frontmatter property → CSS snippet).

4. **`icon::` como propiedad de la template page**. Cuando un bloque activa esa template, el bullet se reemplaza por ese emoji/icon. Decora sin requerir layout de card.

5. **Dual-read transitorio**: bloques con `type:: reference` o `type:: documentacion` siguen funcionando con fallback a las cards hardcodeadas, y un `console.warn` indica que la propiedad debe migrarse. Sin breakage de datos existentes.

6. **El usuario no toca React**. Crear una template page es la única manera de definir un nuevo tipo de card. El sistema es abierto por datos.

### Lo que se preserva

- La utilidad visual de ReferenceCard (metas, open, copy).
- La utilidad de ContentCard (colapsabilidad).
- La existencia del `template/` namespace para page-level cloning.
- El endpoint `POST /api/v1/pages/from-template` para crear páginas nuevas a partir de templates.

### Lo que se elimina

- `ReferenceCard.tsx` y `ContentCard.tsx` como componentes hardcodeados — reemplazados por un `CardRenderer` data-driven.
- `getBlockType()` con switch sobre `type::` — reemplazado por lookup de template page.
- Los 3 botones hardcodeados del EmptyState (Add first block, + Reference, + Documentation) — reemplazados por un template picker (Fase 2) o, en V1, por botones que crean bloques con `template::` apuntando a las 2 templates default.

## Considered Options

1. **Eliminar las cards por completo** — rejected: el usuario pierde la utilidad de diferenciación visual, metas, acciones, colapsabilidad. Regresión funcional.
2. **Mantener cards hardcodeadas y agregar `template::` como propiedad paralela sin efecto visual** — rejected: el código tiene dos sistemas discriminantes para la misma cosa. Deuda técnica inmediata.
3. **Cards como entidades separadas con tabla propia** — rejected: CONTEXT.md define Template como "bloque con propiedad template:: nombre", no como entidad separada. Una tabla adicional introduce JOINs, migraciones, y fragmenta el modelo de propiedades. La evidencia de la ronda de auto-grill (docs/grill/2026-06-03-template-cards.report.md §2) confirmó que el modelo property-based alinea con ADR-0003, ADR-0006, y el CONTEXT.md.
4. **Cards data-driven vía `card-shape::` en template page** — accepted: preserva la utilidad de las cards, hace el sistema extensible por el usuario, y mantiene el modelo de propiedad-block canónico.

## Consequences

### Positivas

- El usuario puede crear infinitos tipos de cards sin tocar código
- Los agentes AI descubren templates via `quilt_list_templates` (Fase 2)
- El esquema de cada template es descubrible via `quilt_get_template_schema` (Fase 2)
- Los estilos son definibles por el usuario via CSS snippets + `cssclass::`
- La propiedad `template::` se alinea con CONTEXT.md y ADR-0006
- La migración desde `type::` es no-breaking (dual-read)

### Negativas

- El `CardRenderer` es más complejo que los 2 componentes originales (interpreta un shape enum en vez de renderizar directamente)
- La validación de que una template page tiene un `card-shape::` válido no existe en V1 (runtime warn)
- La inconsistencia pre-existente del nombre `documentacion` (sin tilde) se arrastra al nuevo sistema como `card-shape:: content` (nombre corregido, en inglés)

### Trade-offs aceptados

- **Discrimidador por shape en vez de tipo libre**: el shape es un enum cerrado de 3 valores en V1. Extensibilidad via nuevos templates, no via nuevos shapes. Esto es deliberado: shapes sin código que los renderice no tienen sentido. Si un usuario quiere una card con comportamiento no cubierto, debe pedir el shape (o contribuirlo).
- **`card-shape::` es opcional con default a `inline`**: si una template page no declara shape, los bloques que la usan se renderizan como bloques normales con la decoración del icono y CSS class. Esto preserva el uso de templates como "categorizadores" sin forzar card visual.
- **Migración dual-read sin tool de migración**: bloques con `type::` siguen funcionando con warn. La migración es progresiva: cuando el usuario edita un bloque, puede actualizarlo al nuevo formato. No hay script de migración masiva porque no hay datos que migrar (las propiedades viven en el mismo `properties` JSON).

## Migration

1. Quilt detecta la ausencia de `template` en `BUILTIN_PROPERTIES` y la agrega con un commit en `quilt-core/src/schema/properties.rs`
2. Quilt elimina `ReferenceCard.tsx` y `ContentCard.tsx`, introduce `CardRenderer.tsx` que lee `card-shape::` de la template page correspondiente
3. `getBlockType()` se reescribe: lee `template::` del bloque, busca la template page, lee `card-shape::` de esa página, y retorna el shape. Fallback a `type::` con warn para bloques legacy.
4. Las templates default `template/reference` y `template/documentation` se crean via seed (o se documenta al usuario cómo crearlas)
5. Los handlers `handleNewReferenceBlock` y `handleNewDocumentacionBlock` se renombran a `handleNewCardBlock(shape: CardShape)` y crean bloques con `template:: <shape-name>`

## Out of scope (deferred to V2+)

- Herencia de templates (`template:: extends-base`)
- Tipos de propiedades tipados en el schema del template (date, enum, ref)
- Live queries dentro de templates
- Expresiones (`{{#if}}`, `{{#each}}`)
- Versionado de templates (snapshot inmutable al aplicar)
- Composición (`{{include template/header}}`)
- Bulk apply de templates
- Template marketplace via MCP
- Validación server-side del schema del template
- Visual template editor

## References

- docs/grill/2026-06-03-template-cards.report.md — auto-grill Pass 1 report
- CONTEXT.md:35-37 — definición de Template
- ADR-0003:5 — "Los templates actúan como contrato"
- ADR-0006:17 — `template` es propiedad de primera clase en v1
- crates/quilt-domain/src/entities/page.rs:160-167 — `Page::is_template_name`
- crates/quilt-server/src/handlers/pages.rs:425-583 — `create_page_from_template`
- quilt-ui/src/features/outliner-tiptap/PageView.tsx:33-39 — current `getBlockType` (to be replaced)
- quilt-ui/src/shared/components/ReferenceCard.tsx, ContentCard.tsx — current card components (to be deleted)
