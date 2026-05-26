# Outliner Professional Baseline

Objetivo: superar a Logseq empezando por un **Outliner** sólido, sin agentes ni LLMs embebidos en la aplicación, y manteniendo MCP como única interfaz para agentes externos.

## Decisiones fijadas

- Quilt supera a Logseq primero en el núcleo del **Outliner**; los diferenciadores extra vienen después.
- No se usa CRDT como eje de diferenciación de esta fase.
- La aplicación sigue siendo **MCP-first**: nada de agentes in-app ni clientes LLM embebidos.
- El modelo canónico sigue siendo **Grafo + Página + Bloque + Propiedad + Outliner**.
- La fuente de verdad sigue siendo el dominio de Quilt; el motor de edición es un adaptador de interacción.
- La granularidad elegida es **un editor textual por Bloque**, coordinado por el Outliner a nivel de **Página**.
- Las propiedades se editan **solo inline**.
- El soporte semántico inline se implementa con un **parser incremental unificado por Bloque**.
- Undo/redo debe ser una **historia unificada de intención de usuario**.

## Baseline profesional acordado (fase B)

Para considerar que Quilt ya no está en modo demo y sí en outliner profesional, el baseline incluye:

- undo/redo
- `[[Página]]` con autocomplete
- `((Bloque))`
- slash commands
- drag & drop
- navegación completa por teclado
- sidebar derecho con backlinks y unlinked references
- propiedades inline editables
- journals funcionales

## Orden de implementación acordado

1. motor serio por **Bloque** + parser semántico unificado
2. refs/tags/properties con autocomplete
3. undo/redo
4. navegación completa por teclado
5. sidebar derecho funcional
6. slash commands
7. propiedades inline fuertes
8. drag & drop
9. journals funcionales

Regla de priorización: **primero semántica, luego confianza, luego velocidad**.

## Propiedades de primera clase v1

- `status`
- `priority`
- `scheduled`
- `deadline`
- `tags`
- `template`
- `created_by`

`journal` queda como semántica de **Página**, no como propiedad general de **Bloque**.

### Ciclo canónico de `status` en v1

- `TODO`
- `DOING`
- `DONE`

Atajo acordado: `Mod+Enter` debe ciclar `TODO → DOING → DONE → TODO`, replicando el comportamiento de referencia de Logseq.

### Normalización de tags en v1

- `#tag` es azúcar sintáctico de entrada.
- `tags::` es la representación explícita inline.
- Ambas formas deben normalizarse al mismo modelo interno: la **Propiedad** `tags`.

### Normalización de refs en v1

- `[[Página]]` debe mantenerse como sintaxis visible, pero normalizarse a una ref estructurada del dominio.
- `((Bloque))` debe mantenerse como sintaxis visible, pero normalizarse a una ref estructurada del dominio.
- Las refs no deben sobrevivir solo como texto decorado: queries, backlinks, MCP y navegación deben apoyarse en la representación estructurada.

## Frontera entre dominio y motor

### Operaciones del Outliner (dominio)

- crear/borrar **Bloque**
- split / merge
- indent / outdent
- mover / reorder
- colapsar / expandir
- insertar/eliminar refs
- editar **Propiedades**
- cambiar marker / priority / status
- mover entre **Páginas**

### Detalles locales del motor de edición

- cursor
- selección dentro del contenido de un bloque
- composición IME
- popup de autocomplete/slash
- decoraciones visuales
- hover preview
- layout de texto

Regla: toda acción que cambie estructura, refs, propiedades o semántica del **Bloque** debe resolverse como operación del Outliner.
