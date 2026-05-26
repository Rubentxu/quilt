# ADR-0006: El motor de edición es un adaptador por Bloque; el Outliner sigue siendo la fuente de verdad

Status: accepted

Para superar a Logseq sin romper el modelo de Quilt, el producto mantiene **Página**, **Bloque**, **Propiedad** y **Outliner** como fuente de verdad del dominio. El `contenteditable` actual se reemplaza por un motor de edición serio por **Bloque**, coordinado a nivel de **Página**, pero toda operación que cambie estructura, refs, propiedades o semántica se resuelve como operación del Outliner. También se adopta un parser incremental unificado por **Bloque** para `property:: value`, `[[Página]]`, `((Bloque))` y `#tag`, y se mantiene la restricción MCP-first: sin agentes ni LLMs embebidos dentro de la aplicación.

## Considered Options

1. **Seguir con `contenteditable`** — rejected: cursor, selección, undo/redo y semántica inline quedarían frágiles.
2. **Mover la fuente de verdad al editor** — rejected: diluye el dominio de bloques/páginas y rompe la alineación con MCP.
3. **Motor serio por Bloque sobre el dominio existente** — accepted: preserva el modelo inspirado en Logseq y mejora la UX sin invertir dependencias.

## Consequences

- Undo/redo debe modelarse como historia unificada de intenciones del Outliner, no solo del texto.
- Las propiedades se editan inline, pero siguen siendo **Propiedades** tipadas del dominio.
- El núcleo v1 de propiedades de primera clase es: `status`, `priority`, `scheduled`, `deadline`, `tags`, `template`, `created_by`.
- `journal` sigue siendo semántica de **Página**, no una propiedad general de **Bloque**.
