# User Stories — Edición de Bloques

> **Proyecto**: Logseq
> **Generado por**: reversa-writer
> **Fecha**: 2026-05-02
> **Nivel**: detalhado
> **Escala**: 🟢 CONFIRMADO | 🟡 INFERIDO | 🔴 LACUNA

---

## Flujo 1: Crear Bloque

### US-EDB-01: Crear un bloque en una página existente 🟢 CONFIRMADO

**Contexto**: Un usuario quiere añadir un nuevo bloque de texto a una página que ya existe en su grafo de conocimiento.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `insert-blocks`, `frontend/handler/editor.cljs` → `insert-new-block!`

```gherkin
Dado que el usuario está en una página existente "Reuniones"
  Y el cursor se encuentra al final del último bloque
Cuando el usuario presiona Enter y escribe "Revisar presupuesto Q2"
Então se crea un nuevo bloque con el contenido "Revisar presupuesto Q2"
  Y el bloque recibe un UUID único inmutable generado automáticamente
  Y el bloque se asigna como sibling del bloque anterior (mismo nivel)
  Y el bloque recibe un orden lexicográfico posicionado correctamente entre siblings
  Y el `:block/created-at` y `:block/updated-at` se establecen al timestamp actual
  Y la página padre se registra en `:block/page`
```

---

### US-EDB-02: Crear un bloque indentado (hijo de otro bloque) 🟢 CONFIRMADO

**Contexto**: Un usuario quiere crear un bloque anidado bajo otro bloque existente, formando una jerarquía.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `insert-blocks` con `sibling? = false`, `frontend/handler/editor.cljs`

```gherkin
Dado que el usuario tiene un bloque "Tareas del proyecto" con varios hijos
  Y el cursor está al final del último hijo
Cuando el usuario presiona Tab y luego escribe "Actualizar documentación"
Então se crea un nuevo bloque indentado bajo "Tareas del proyecto"
  Y el `:block/parent` del nuevo bloque apunta al bloque "Tareas del proyecto"
  Y el `:block/level` es level(parent) + 1
  Y el bloque padre se expande automáticamente si estaba colapsado
  Y el nuevo bloque hereda el formato (:markdown u :org) de la página padre
```

---

### US-EDB-03: Crear un bloque usando el comando slash (/) con autocompletado 🟢 CONFIRMADO

**Contexto**: El usuario usa el comando slash para insertar bloques especiales como headings, tareas, citas o queries.

**Rastreabilidad**: `frontend/components/editor.cljs` → `commands` (autocompletado de comandos slash), `filter-commands`

```gherkin
Dado que el usuario está editando un bloque vacío
Cuando el usuario escribe "/" seguido de "TOD"
Então se muestra un menú de autocompletado con comandos que coinciden
  Y el comando "TODO" aparece entre las opciones
  Y al seleccionar "TODO", el bloque se convierte en un bloque de tarea con marker TODO
  Y el marker se asigna como propiedad `:logseq.property/status.todo`
  Y el bloque se guarda automáticamente con el tipo de bloque correspondiente
```

---

### US-EDB-04: Crear un bloque con referencia a otra página [[wiki-link]] 🟢 CONFIRMADO

**Contexto**: El usuario escribe una referencia a otra página usando la sintaxis `[[nombre-pagina]]`.

**Rastreabilidad**: `deps/graph-parser/src/logseq/graph_parser/block.cljs` → `get-page-reference`, `frontend/components/editor.cljs` → `search-pages`

```gherkin
Dado que el usuario está creando un nuevo bloque
Cuando el usuario escribe "Ver [[Meeting Notes]] para más detalles"
Então se crea una referencia a la página "Meeting Notes" en `:block/refs`
  Y si la página "Meeting Notes" ya existe, se enlaza correctamente
  Y si la página "Meeting Notes" NO existe, se crea un placeholder con UUID generado
  Y la referencia se renderiza como un enlace clickable en la UI
  Y la página "Meeting Notes" recibe un backlink automático hacia el bloque creado
```

---

### US-EDB-05: Crear un bloque en una página nueva (on-the-fly) 🟡 INFERIDO

**Contexto**: El usuario hace referencia a una página que no existe y Logseq la crea automáticamente como placeholder.

**Rastreabilidad**: `deps/graph-parser/src/logseq/graph_parser/block.cljs` → `page-name->map`, `with-ref-pages`

```gherkin
Dado que el usuario escribe "Ver [[Nueva Idea]] para explorar"
  Y la página "Nueva Idea" no existe en el grafo
Cuando el bloque se guarda
Então se crea una página placeholder con nombre "nueva idea" (normalizado a lowercase)
  Y la página recibe un UUID generado automáticamente
  Y la página se marca para creación diferida (no se crea el archivo inmediatamente)
  Y el bloque actual establece referencia `:block/refs` hacia la nueva página
```

---

## Flujo 2: Editar Bloque

### US-EDB-06: Editar el contenido de un bloque existente 🟢 CONFIRMADO

**Contexto**: El usuario modifica el texto de un bloque ya existente en el outliner.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `save-block`, `frontend/handler/editor.cljs` → `edit-block!`

```gherkin
Dado que existe un bloque con contenido "Revisar presupuesto"
  Y el bloque tiene UUID "550e8400-e29b-41d4-a716-446655440000"
Cuando el usuario hace clic en el bloque, edita el texto a "Revisar presupuesto Q2 2026" y sale del modo edición
Então el bloque se guarda con el nuevo contenido
  Y el `:block/updated-at` se actualiza al timestamp actual
  Y el UUID del bloque permanece INALTERADO (validación de inmutabilidad)
  Y los metadatos del bloque (`:block/created-at`, `:block/page`, `:block/parent`) no se modifican
  Y si el bloque tiene referencias, se reextraen del nuevo contenido
```

---

### US-EDB-07: Editar un bloque con propiedades (property drawer) 🟢 CONFIRMADO

**Contexto**: El usuario modifica propiedades del bloque como `title::`, `tags::`, `priority::` o `schedule::`.

**Rastreabilidad**: `deps/graph-parser/src/logseq/graph_parser/block.cljs` → `extract-properties`, `frontend/db/model.cljs` → `get-block-by-uuid`

```gherkin
Dado que un bloque tiene propiedades:
  ```
  title:: Notas de Diseño
  priority:: A
  ```
Cuando el usuario edita el bloque y cambia `priority:: A` a `priority:: B`
Então la propiedad `priority` se actualiza al valor `:logseq.priority/b`
  Y la propiedad `title` permanece sin cambios
  Y `:block/properties` se actualiza en la transacción de DataScript
  Y los timestamps `:block/created-at` y `:block/updated-at` se preservan correctamente
```

---

### US-EDB-08: Editar bloque con formato markdown (bold, italic, code) 🟢 CONFIRMADO

**Contexto**: El usuario aplica formato enriquecido al contenido del bloque usando sintaxis markdown.

**Rastreabilidad**: `deps/graph-parser/src/logseq/graph_parser/block.cljs` → `extract-blocks` (procesamiento de AST), `frontend/format/mldoc.cljs` → `->edn`

```gherkin
Dado que el usuario edita un bloque y escribe:
  ```
  **Importante**: Revisar la `API` de *autenticación*
  ```
Cuando el bloque se guarda
Então `Important` se almacena con marcado de bold (AST: Strong)
  Y `API` se almacena con marcado de código inline (AST: Code)
  Y `autenticación` se almacena con marcado de italic (AST: Emphasis)
  Y el contenido raw se preserva en `:block/content` con la sintaxis markdown original
```

---

### US-EDB-09: Editar bloque desde diferentes vistas (page view, sidebar, referenced block) 🟡 INFERIDO

**Contexto**: El usuario edita un bloque que está siendo mostrado en múltiples contextos simultáneamente (vista de página, sidebar derecho, bloque embebido).

**Rastreabilidad**: `frontend/components/block.cljs`, `frontend/components/editor.cljs` → `node-render`

```gherkin
Dado que un bloque "Notas Importantes" se muestra en:
  - La vista principal de la página "Proyecto Alpha"
  - El sidebar derecho como referencia
  - Embebido en otra página via {{embed}}
Cuando el usuario edita el bloque desde la vista principal y cambia el texto a "Notas Críticas"
Então el cambio se refleja instantáneamente en todas las vistas
  Y el sidebar derecho muestra el contenido actualizado
  Y el bloque embebido en la otra página se refresca
  Y solo existe una copia en DataScript (no hay duplicación)
```

---

### US-EDB-10: Cancelar edición de bloque (ESC) revierte cambios 🟢 CONFIRMADO

**Contexto**: El usuario inicia la edición de un bloque pero decide cancelarla antes de guardar.

**Rastreabilidad**: `frontend/state.cljs` → `:editor/editing?`, `frontend/components/editor.cljs`

```gherkin
Dado que un bloque tiene contenido original "Tareas Pendientes"
  Y el usuario hace clic para editarlo
  Y cambia el contenido a "Tareas Completadas"
Cuando el usuario presiona ESC antes de que se guarde automáticamente
Então el bloque retorna a su contenido original "Tareas Pendientes"
  Y no se genera ninguna transacción en DataScript
  Y el estado `:editor/editing?` vuelve a nil
  Y el cursor sale del modo edición
```

---

## Flujo 3: Guardar Bloque

### US-EDB-11: Guardado automático al perder el foco (on-blur) 🟢 CONFIRMADO

**Contexto**: El sistema guarda automáticamente el bloque cuando el usuario sale del modo edición.

**Rastreabilidad**: `frontend/handler/editor.cljs` → `save-current-block!`, `deps/outliner/src/logseq/outliner/core.cljs` → `-save`

```gherkin
Dado que el usuario está editando un bloque con contenido "Draft de propuesta"
Cuando el usuario hace clic fuera del bloque (pierde el foco)
Então el bloque se guarda automáticamente en DataScript
  Y se ejecuta `save-current-block!` vía el event loop
  Y el contenido se persiste en el archivo .md/.org correspondiente
  Y los hooks `hook:db-tx` y `hook:block-changes` se disparan para plugins
  Y el índice de búsqueda se actualiza incrementalmente con `transact-blocks!`
```

---

### US-EDB-12: Guardar bloque con validación de integridad 🟢 CONFIRMADO

**Contexto**: El sistema valida la integridad del bloque antes de persistirlo en DataScript.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `save-block` (validación UUID, built-in), `frontend/handler/editor.cljs`

```gherkin
Dado que existe un bloque con UUID "550e8400-..."
  Y el bloque NO es una entidad built-in
Cuando el sistema guarda el bloque
Então se valida que el UUID no ha cambiado (comparación con entidad existente en DB)
  Y se verifica que el bloque no es built-in (páginas como "Contents", "logseq/custom.css")
  Y los campos temporales (`:block/temp-id`, etc.) se eliminan antes de la transacción
  Y el `:block/updated-at` se actualiza al timestamp actual
  Y se corrigen los tag-IDs (`fix-tag-ids`) para mantener consistencia referencial
```

---

### US-EDB-13: Guardar bloque que es una página (validación de título) 🟢 CONFIRMADO

**Contexto**: Cuando el bloque guardado es una página (tiene `:block/name`), se validan reglas de título.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → validación de título en `save-block`, `frontend/handler/page.cljs`

```gherkin
Dado que un bloque de tipo página tiene `:block/name` = "Mi Página"
  Y el nombre no contiene caracteres prohibidos (/ # ? : | < > * " \)
  Y el nombre no es vacío ni solo números
Cuando se guarda el bloque
Então el título se normaliza a lowercase: "mi página"
  Y el bloque se persiste correctamente en DataScript
  Y el archivo correspondiente se crea/actualiza en el filesystem
```

---

### US-EDB-14: Guardar bloque con título inválido (rechazo) 🟢 CONFIRMADO

**Contexto**: El sistema rechaza guardar un bloque cuyo título contiene caracteres prohibidos.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → validación de caracteres prohibidos, `frontend/handler/editor.cljs`

```gherkin
Dado que el usuario intenta guardar un bloque con título "Notas/2026" (contiene /)
Cuando el sistema intenta validar el título
Então la validación falla porque "/" es un carácter prohibido
  Y se lanza una excepción `ex-info` con mensaje descriptivo
  Y el bloque NO se persiste en DataScript
  Y se muestra un mensaje de error al usuario
  Y el bloque permanece en modo edición para que el usuario corrija el título
```

---

### US-EDB-15: Guardar bloque con propiedades tipo closed-value 🟢 CONFIRMADO

**Contexto**: El sistema valida que las propiedades de tipo cerrado (task markers, priority) tengan valores predefinidos válidos.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs`, `frontend/db/model.cljs`, `frontend/handler/editor.cljs`

```gherkin
Dado que un bloque tiene la propiedad `status` con valor `:logseq.property/status.todo`
  Y el usuario cambia el marker a DONE
Cuando el bloque se guarda
Então la propiedad `status` se actualiza a `:logseq.property/status.done`
  Y el valor debe ser uno de los closed-values: TODO, NOW, LATER, DONE, CANCELLED
  Y el `:block/updated-at` refleja el momento del cambio
  Y los queries reactivos que filtran por este marker se refrescan automáticamente
```

---

## Flujo 4: Eliminar Bloque

### US-EDB-16: Eliminar un bloque individual 🟢 CONFIRMADO

**Contexto**: El usuario elimina un bloque del outliner usando Backspace o el comando de eliminar.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `delete-blocks`, `frontend/handler/editor.cljs` → `delete-block!`

```gherkin
Dado que existe un bloque "Nota temporal" en la página "Proyecto"
  Y el bloque no es built-in
  Y el bloque no tiene hijos
Cuando el usuario selecciona el bloque y presiona Backspace (estando vacío)
Então el bloque se elimina de DataScript vía `retractEntity`
  Y el bloque desaparece de la vista del outliner
  Y los siblings se reordenan automáticamente (sin gaps)
  Y el índice de búsqueda se actualiza para eliminar el bloque
  Y los hooks `hook:db-tx` y `hook:block-changes` se disparan
```

---

### US-EDB-17: Eliminar un bloque con hijos (cascada) 🟢 CONFIRMADO

**Contexto**: Al eliminar un bloque padre, todos sus hijos se eliminan en cascada.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `delete-blocks` con `filter-top-level-blocks`, transacción batch

```gherkin
Dado que existe un bloque "Sección Principal" con 3 hijos:
  - "Subsección A"
  - "Subsección B"
  - "Subsección C" (que a su vez tiene 2 nietos)
Cuando el usuario elimina "Sección Principal"
Então "Sección Principal" se marca como top-level block para eliminación
  Y los 3 hijos directos se eliminan implícitamente por `retractEntity` de DataScript
  Y los 2 nietos también se eliminan en cascada
  Y el batch de transacción solo incluye el bloque top-level (no los hijos)
  Y el índice de búsqueda elimina todos los bloques afectados
```

---

### US-EDB-18: Intentar eliminar un bloque built-in (rechazo) 🟢 CONFIRMADO

**Contexto**: El sistema impide la eliminación de bloques que pertenecen a páginas built-in del sistema.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → validación anti-built-in, `frontend/handler/editor.cljs`

```gherkin
Dado que existe un bloque en la página built-in "Contents"
  O un bloque en "logseq/custom.css"
Cuando el usuario intenta eliminar ese bloque
Então la operación es rechazada
  Y se lanza excepción `ex-info` con tipo `:built-in-entity`
  Y el mensaje contiene "Cannot modify built-in entity"
  Y el bloque permanece intacto en la UI y en DataScript
```

---

### US-EDB-19: Eliminar el último bloque de una página (página huérfana → recycle) 🟡 INFERIDO

**Contexto**: Cuando se elimina el último bloque de una página, la página queda huérfana y se envía a recycle (papelera).

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `delete-blocks` (orphan handling), recycle logic

```gherkin
Dado que una página "Temporal" contiene un único bloque "Desechar"
Cuando el usuario elimina el bloque "Desechar"
Então el bloque se elimina
  Y la página "Temporal" queda sin bloques (huérfana)
  Y la página se envía a recycle (papelera), NO se elimina permanentemente
  Y la página puede ser restaurada posteriormente desde recycle
  Y si la página es un journal, se preserva la estructura de `:block/journal-day`
  Y las referencias a esta página desde otros bloques se limpian (clean-orphaned-refs)
```

---

### US-EDB-20: Eliminar múltiples bloques seleccionados (bulk delete) 🟡 INFERIDO

**Contexto**: El usuario selecciona varios bloques no consecutivos y los elimina en una sola operación.

**Rastreabilidad**: `deps/outliner/src/logseq/outliner/core.cljs` → `delete-blocks` con `sort-by-order`, `non-consecutive-blocks->vec-tree`

```gherkin
Dado que el usuario tiene seleccionados 3 bloques no consecutivos en la misma página:
  - Bloque A (posición 1)
  - Bloque C (posición 3)
  - Bloque E (posición 5)
Cuando el usuario ejecuta la acción de eliminar
Então los bloques se ordenan por `:block/order` para procesamiento
  Y se filtran solo los top-level blocks del grupo
  Y cada bloque se valida contra la regla anti-built-in
  Y se ejecuta una transacción batch eliminando todos los bloques seleccionados
  Y los bloques entre los seleccionados (B en posición 2, D en posición 4) no se afectan
```

---

## Cenários de Borda

### EDB-BORDE-1: Crear bloque con más de 10,000 caracteres 🟡 INFERIDO

**Contexto**: Un usuario pega un texto extremadamente largo como contenido de un solo bloque.

**Comportamiento esperado**:
- El bloque se crea normalmente sin límite de longitud explícito en el código
- DataScript almacena el contenido completo como string
- La UI puede experimentar lentitud en el renderizado de bloques muy largos
- El índice de búsqueda indexa el contenido completo

---

### EDB-BORDE-2: Editar un bloque mientras se está sincronizando (race condition sync) 🟡 INFERIDO

**Contexto**: El usuario edita un bloque localmente mientras una sincronización remota está actualizando ese mismo bloque.

**Comportamiento esperado**:
- La edición local se procesa inmediatamente
- Si llega una transacción remota conflictiva durante la edición, se detecta vía checksum
- El sistema de resolución de conflictos (sync) compara timestamps y aplica la política configurada (local wins vs remote wins)
- El estado `:editor/editing?` podría invalidarse si la entidad subyacente cambia

---

### EDB-BORDE-3: Crear bloque con referencia a un bloque que aún no existe (forward ref) 🔴 LACUNA

**Contexto**: El usuario escribe `((block-ref))` apuntando a un UUID que aún no ha sido creado.

**Comportamiento esperado**:
- El comportamiento exacto no está confirmado en el código analizado
- Posiblemente se crea una referencia rota (dangling ref) que se muestra como "Block not found"
- Podría resolverse cuando el bloque referenciado se cree posteriormente

---

## Resumen de cobertura

| Flujo | Escenarios | Confianza predominante |
|-------|-----------|----------------------|
| Crear Bloque | 5 | 🟢 CONFIRMADO (4), 🟡 INFERIDO (1) |
| Editar Bloque | 5 | 🟢 CONFIRMADO (4), 🟡 INFERIDO (1) |
| Guardar Bloque | 5 | 🟢 CONFIRMADO (5) |
| Eliminar Bloque | 5 | 🟢 CONFIRMADO (3), 🟡 INFERIDO (2) |
| Cenários de Borda | 3 | 🟡 INFERIDO (2), 🔴 LACUNA (1) |

---

*Documento generado automáticamente por Reversa Writer*
