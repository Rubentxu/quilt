# Backlog Inicial — Outliner Profesional

## Épica 1 — Motor de edición por Bloque y parser semántico

### Issue 1.1 — Reemplazar `contenteditable` por motor serio por Bloque
- **Objetivo**: introducir un adaptador de edición robusto por bloque.
- **Alcance**: refactor de `components/block_editor.rs`, input y selección.
- **Dependencias**: ninguna.
- **Aceptación**:
  - Enter/Shift+Enter/Tab/Shift+Tab se comportan como Logseq.
  - Cursor, selección e IME son estables.
- **Archivos probables**:
  - `crates/quilt-ui/src/components/block_editor.rs`
  - `crates/quilt-ui/src/editor/*`

### Issue 1.2 — Parser incremental unificado por Bloque
- **Objetivo**: parsear `[[Página]]`, `((Bloque))`, `#tag`, `property:: value`.
- **Alcance**: semántica estructurada preservando sintaxis visible.
- **Dependencias**: 1.1.
- **Aceptación**:
  - refs y properties se extraen de forma idempotente.
  - `#tag` normaliza a `tags`.
- **Archivos probables**:
  - `crates/quilt-ui/src/parser/*`

## Épica 2 — Refs, tags y properties con autocomplete

### Issue 2.1 — Autocomplete para `[[Página]]`
- **Objetivo**: dropdown de páginas con búsqueda fuzzy.
- **Dependencias**: 1.1.
- **Aceptación**:
  - `[[` abre autocomplete.
  - Enter inserta la página.

### Issue 2.2 — Autocomplete para `((Bloque))`
- **Objetivo**: dropdown de bloques con preview.
- **Dependencias**: 1.1.
- **Aceptación**:
  - `((` abre autocomplete.
  - Enter inserta el bloque seleccionado.

### Issue 2.3 — Normalización y autocomplete de `#tag`
- **Objetivo**: unificar `#tag` con `tags::`.
- **Dependencias**: 1.2.
- **Aceptación**:
  - `#tag` y `tags::` son equivalentes en el modelo.

### Issue 2.4 — Autocomplete de propiedades inline
- **Objetivo**: soporte tipado para `status`, `priority`, `scheduled`, `deadline`, `tags`, `template`, `created_by`.
- **Dependencias**: 1.2.
- **Aceptación**:
  - `status::` sugiere `TODO`, `DOING`, `DONE`.
  - fechas tienen selector adecuado.

## Épica 3 — Undo/Redo unificado por intención

### Issue 3.1 — HistoryStack del Outliner
- **Objetivo**: undo/redo por operación semántica.
- **Dependencias**: 1.1, 2.x.
- **Aceptación**:
  - `Mod+Z` y `Mod+Shift+Z` revierten texto, split, indent, merge, props y moves.
- **Archivos probables**:
  - `crates/quilt-ui/src/outliner/history.rs`

## Épica 4 — Navegación completa por teclado

### Issue 4.1 — Navegación entre bloques
- **Objetivo**: arrow keys, foco y selección de bloques.
- **Dependencias**: 1.1.
- **Aceptación**:
  - navegación keyboard-first sin ratón.

### Issue 4.2 — Shortcuts de edición
- **Objetivo**: bold/italic y otros atajos de texto.
- **Dependencias**: 1.1.

### Issue 4.3 — Shortcuts estructurales
- **Objetivo**: `Mod+Enter`, mover bloques, zoom, collapse/expand.
- **Dependencias**: 1.1, 3.1.

## Épica 5 — Sidebar derecho funcional

### Issue 5.1 — Linked References
- **Objetivo**: backlinks reales por página.
- **Dependencias**: 1.2.
- **Aceptación**:
  - listado navegable de backlinks.

### Issue 5.2 — Unlinked References
- **Objetivo**: detectar menciones no enlazadas.
- **Dependencias**: 5.1.

### Issue 5.3 — Múltiples paneles en sidebar
- **Objetivo**: stacked panels en sidebar derecho.
- **Dependencias**: 5.1.

## Épica 6 — Slash commands

### Issue 6.1 — Sistema `/`
- **Objetivo**: menú de comandos alineado con el dominio.
- **Dependencias**: 1.1, 2.1, 2.2.
- **Aceptación**:
  - `/` abre menú.
  - Enter ejecuta el comando.

## Épica 7 — Propiedades inline fuertes

### Issue 7.1 — Renderizado visual inline
- **Objetivo**: representar properties v1 de forma clara sin salir del flujo inline.
- **Dependencias**: 1.2.

### Issue 7.2 — Edición directa de properties
- **Objetivo**: editar status, priority, fechas y tags desde su representación inline.
- **Dependencias**: 2.3, 2.4, 7.1.

## Épica 8 — Drag & drop

### Issue 8.1 — Drag & drop de bloques
- **Objetivo**: reordenamiento y reparenting con feedback visual.
- **Dependencias**: 1.1, 3.1.

### Issue 8.2 — Drag & drop de favoritos
- **Objetivo**: reordenar favoritos del sidebar izquierdo.
- **Dependencias**: ninguna.

## Épica 9 — Journals funcionales

### Issue 9.1 — Navegación de journals
- **Objetivo**: journals diarios operativos.
- **Dependencias**: 1.1, 4.1.

### Issue 9.2 — Calendario en sidebar
- **Objetivo**: navegación por calendario.
- **Dependencias**: 9.1.

## Priorización global

### MUST
- 1.1
- 1.2
- 2.1
- 2.2
- 2.3
- 2.4
- 3.1
- 4.1
- 5.1
- 6.1
- 9.1

### SHOULD
- 4.2
- 4.3
- 5.2
- 5.3
- 7.1
- 7.2
- 8.1

### COULD
- 8.2
- 9.2
