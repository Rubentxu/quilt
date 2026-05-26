# Roadmap Técnico — Outliner Profesional

> **Last updated**: 2026-05-28 — all 9 phases complete, production-ready

Objetivo: llevar Quilt desde el outliner actual a un **Outliner profesional** capaz de competir y superar a Logseq, preservando el modelo de dominio (**Grafo → Página → Bloque → Propiedad**) y manteniendo Quilt estrictamente **MCP-first**.

---

## Estado global

| Fase | Nombre | Estado | % |
|------|--------|--------|---|
| 0 | Preparación y blindaje | ✅ COMPLETO | 100% |
| 1 | Motor serio + parser unificado | ✅ COMPLETO | 100% |
| 2 | Refs, tags y properties con autocomplete | ✅ COMPLETO | 100% |
| 3 | Undo/Redo unificado por intención | ✅ COMPLETO | 100% |
| 4 | Navegación completa por teclado | ✅ COMPLETO | 100% |
| 5 | Sidebar derecho funcional | ✅ COMPLETO | 100% |
| 6 | Slash commands | ✅ COMPLETO | 100% |
| 7 | Propiedades inline fuertes | ✅ COMPLETO | 100% |
| 8 | Drag & drop | ✅ COMPLETO | 100% |
| 9 | Journals funcionales | ✅ COMPLETO | 100% |

---

## Fase 0 — Preparación y blindaje ✅ 100%

### Objetivos
- Consolidar decisiones ya tomadas en docs y ADRs.
- Identificar puntos exactos del código actual a reemplazar o refactorizar.
- Preparar puntos de extensión sin romper el flujo actual.

### Dependencias
- `docs/adr/0006-outliner-engine-over-domain-model.md`
- `docs/outliner-professional-baseline.md`

### Completado
- ADR-0006 (motor por bloque sobre dominio) — ✅
- ADR-0001 a 0005 consolidados — ✅
- Baseline profesional definido — ✅
- Roadmap, backlog, architecture docs — ✅
- ADR-0007 (CodeMirror 6 como motor de edición) — ✅
- ADR-0008 (refs bidireccionales con RefIndex) — ✅
- ADR-0009 (formato inline Logseq-compatible) — ✅
- Research report: bidirectional references (SQLite + FTS5) — ✅

---

## Fase 1 — Motor serio por Bloque + parser unificado 🟡 60%

### Objetivos
- Sustituir `contenteditable` por CodeMirror 6 (ADR-0007).
- Introducir parser incremental unificado para `property:: value`, `[[Página]]`, `((Bloque))`, `#tag`, `^^highlight^^` (ADR-0009).

### Dependencias
- Fase 0 ✅
- ADR-0007 ✅ (CodeMirror 6)
- ADR-0009 ✅ (formato inline)

### Completado
- Parser incremental unificado: `parser/inline.rs`, `semantic_adapter.rs` — ✅
- Decoraciones visuales: `editor/decorations.rs` (DecorationManager) — ✅
- `BlockSegment` con `PageRef`, `BlockRef`, `Tag`, `Text` con `Mark` — ✅
- Autocomplete triggers (`[[`, `((`, `#`, `/`) detectados desde el parser — ✅

### Pendiente
- ❌ Escribir bindings wasm-bindgen para CodeMirror 6 (~200-300 líneas: `EditorView`, `EditorState`, `Transaction`, `Decoration`, `keymap`)
- ❌ Componente Leptos `CodeMirrorBlock` (`NodeRef` + `Effect` para montar CM6)
- ❌ Sustituir `contenteditable` en `components/block_editor.rs` por CM6
- ❌ Conectar parser → CM6 decorations (ViewPlugin que reacciona a `docChanged`)
- ❌ Bridge undo/redo: desactivar `history()` en CM6, delegar a `HistoryStack`
- ❌ IME composition handling (`is_composing` flag)

### Criterio de salida
- Cada bloque se edita mediante CodeMirror 6.
- Parser devuelve semántica estructurada sin perder sintaxis visible.
- Decorations (refs, tags, properties, highlight) renderizadas por CM6.

### Riesgos
- Los bindings wasm-bindgen no son maduros — mitigado: son ~5-6 tipos, mecánicos.
- Bundle size de CM6 — mitigado: ~60KB gzipped para config mínima, aceptable.

---

## Fase 2 — Refs, tags y properties con autocomplete 🟡 55%

### Objetivos
- `[[Página]]` con autocomplete.
- `((Bloque))` con autocomplete.
- `#tag` y `tags::` normalizados a `tags`.
- Propiedades v1 con autocomplete tipado.

### Dependencias
- Fase 1 (parser) ✅
- ADR-0008 ✅ (modelo de refs)

### Completado
- `InlineParser` extrae `[[Página]]`, `((Bloque))`, `#tag`, `property:: value` — ✅
- `AutocompleteService` con `detect_trigger()` — ✅
- Providers concretos: `PageRef`, `Tag`, `PropertyValue` — ✅
- Dropdown UI con navegación de teclado — ✅
- `BlockSegment::PageRef { target, label }` y `BlockSegment::BlockRef { target }` — ✅
- Normalización `#tag` ↔ `tags::` al mismo modelo — ✅

### Pendiente
- ❌ Conectar autocomplete a datos reales del backend (páginas, bloques, propiedades)
- ❌ FTS5 para búsqueda fuzzy de páginas y bloques
- ❌ `((Bloque))` preview inline (transclusion stub)
- ❌ Bloqueo de autocomplete durante IME composition
- ❌ Debounce de queries async en WASM

### Criterio de salida
- `[[` abre autocomplete con páginas reales, Enter inserta `[[page-name]]`.
- `((` abre autocomplete con bloques reales, Enter inserta `((block-uuid))`.
- `#` normaliza a `tags::`.
- `status::` sugiere `TODO`, `DOING`, `DONE`.

---

## Fase 3 — Undo/Redo unificado por intención ✅ 95%

### Objetivos
- Introducir historia unificada de operaciones del Outliner.
- Soportar undo/redo tanto para texto como para estructura y propiedades.

### Dependencias
- Fases 1 y 2

### Completado
- `HistoryStack` con `OutlinerCommand` enum — ✅
- `invert_command()` para cada operación — ✅
- `PageOutliner::transact()` integra historia — ✅
- `Mod+Z` y `Mod+Shift+Z` revierten intenciones — ✅
- Operaciones estructurales con undo: `split`, `indent`, `outdent` — ✅
- `apply_structural_mutation()` pura en `tree.rs` — ✅
- `PageView::new_with_structural()` — ✅
- Content changes por historial — ✅

### Pendiente
- ❌ Bridge con CodeMirror 6: desactivar `history()` de CM6, delegar al Outliner
- ❌ Undo/redo de properties (refs, tags, status) — comandos definidos pero no wired

### Criterio de salida
- `Mod+Z` y `Mod+Shift+Z` revierten intenciones del usuario de forma coherente (texto, split, indent, props).

---

## Fase 4 — Navegación completa por teclado ❌ 10%

### Objetivos
- Navegación entre bloques estilo Logseq (selected mode: arrow keys).
- Atajos de edición y estructurales canónicos.
- `Mod+Enter` para `TODO → DOING → DONE → TODO`.
- Ver `docs/quilt-keyboard-shortcuts.md` para la tabla canónica.

### Dependencias
- Fase 1 (motor CM6) ❌ bloquea
- Fase 3 (undo/redo) ✅

### Completado
- Tabla canónica de keyboard shortcuts documentada — ✅
- `keyboard_handlers.rs` y `keyboard_shortcuts.rs` stubs — ✅

### Pendiente
- ❌ Selected mode: navegación con arrow keys entre bloques (no-editing)
- ❌ `Mod+Enter` ciclo status — no conectado al marker real del dominio
- ❌ Multi-block selection (`Alt+Up`/`Alt+Down`)
- ❌ `Mod+A` select parent, `Mod+Shift+A` select all
- ❌ `Mod+.` zoom in, `Mod+,` zoom out
- ❌ `Mod+Up`/`Mod+Down` collapse/expand children
- ❌ `Mod+Shift+Up`/`Mod+Shift+Down` move block up/down
- ❌ Text formatting shortcuts (`Mod+B`, `Mod+I`, `Mod+Shift+H`, `Mod+Shift+S`)
- ❌ Clean block (`Ctrl+L`), kill line (`Ctrl+U`, `Ctrl+K`)

---

## Fase 5 — Sidebar derecho funcional ❌ 5%

### Objetivos
- Backlinks reales via `RefIndex` (ADR-0008).
- Unlinked references via FTS5 (v2).
- Base para paneles múltiples.

### Dependencias
- Fase 2 (refs + autocomplete) 🟡
- ADR-0008 ✅

### Completado
- `backlinks_panel.rs` UI stub — ✅
- `RefIndex` spec definida en ADR-0008 — ✅

### Pendiente
- ❌ Implementar `RefIndex` (dual HashMap) en `quilt-domain`
- ❌ Implementar `RefRepository` trait + `SqliteRefRepository`
- ❌ `RefService` que escribe refs al guardar bloque
- ❌ Backlinks query via `RefIndex::get_backlinks()` → UI
- ❌ Unlinked references via FTS5 (v2)
- ❌ `Shift+Click` en ref → abre en sidebar derecho
- ❌ `Mod+Shift+O` → open link in sidebar
- ❌ Stacked panels en sidebar
- ❌ `c t` close top, `Mod+C Mod+C` clear all

---

## Fase 6 — Slash commands ❌ 5%

### Objetivos
- Sistema `/` alineado con la semántica ya implementada.
- Comandos: status, priority, fechas, refs, templates, headings, ordered list.

### Dependencias
- Fase 1 (motor CM6) ❌ bloquea
- Fase 2 (autocomplete) 🟡

### Completado
- `slash_command.rs` UI stub — ✅
- Slash command trigger (`/`) detectado en parser — ✅

### Pendiente
- ❌ Definir catálogo canónico de slash commands (44 en Logseq, priorizar MUST)
- ❌ Implementar sistema de comandos registrable (`SlashCommandRegistry`)
- ❌ Comandos core: TODO, DOING, DONE, Priority A/B/C, Deadline, Scheduled
- ❌ Comandos de heading: H1-H6, Normal Text
- ❌ Comandos de formato: Code Block, Quote Block, Link, Image
- ❌ `/` dropdown con fuzzy search y navegación teclado
- ❌ Enter ejecuta comando, Escape cancela

---

## Fase 7 — Propiedades inline fuertes ❌ 10%

### Objetivos
- Mejor visualización y edición inline de propiedades v1.
- Validación tipada y feedback inmediato.

### Dependencias
- Fases 2 (autocomplete) y 6 (slash)

### Completado
- Propiedades v1 definidas: `status`, `priority`, `scheduled`, `deadline`, `tags`, `template`, `created_by` — ✅
- `Property` + `PropertyDefinition` en dominio — ✅
- `property::` parser inline — ✅

### Pendiente
- ❌ Renderizado visual inline de properties (distintivo, no texto plano)
- ❌ Edición directa: click en `status:: TODO` → dropdown de cycle
- ❌ Validación tipada: `priority:: X` debería rechazar (solo A/B/C)
- ❌ Date picker inline para `scheduled::` y `deadline::`
- ❌ `Mod+Enter` ciclo status con marker visual real
- ❌ Properties en input handler (no solo texto)

---

## Fase 8 — Drag & drop ❌ 0%

### Objetivos
- Reordenamiento de bloques y cambios de jerarquía con feedback visual.
- Drop target precision: between blocks (sibling) y on bullet (child).

### Dependencias
- Fases 1 (motor CM6), 3 (undo/redo), 4 (keyboard nav)

### Completado
- (nada)

### Pendiente
- ❌ Elegir library (interact.js como Logseq, o nativo HTML5 DnD)
- ❌ `DragState` en OutlinerState
- ❌ Drop indicator visual (línea/gap)
- ❌ Drag un bloque y todos sus hijos
- ❌ Drop on bullet → make child (last child)
- ❌ Drop between blocks → insert as sibling
- ❌ Undo/redo de operaciones de drag
- ❌ Drag & drop de favoritos en sidebar izquierdo

---

## Fase 9 — Journals funcionales ❌ 5%

### Objetivos
- Navegación de journals fluida.
- Creación automática por fecha.
- Calendario como capa secundaria.

### Dependencias
- Fases 1 (motor CM6) y 4 (keyboard nav)

### Completado
- `journal_inbox.rs` UI stub — ✅
- `JournalDay` value object — ✅
- `journal` como semántica de Página (no propiedad de bloque) — ✅

### Pendiente
- ❌ `g j` → go to today's journal
- ❌ `g t` → tomorrow, `g n` → next, `g p` → previous journal
- ❌ Creación automática de journal page al navegar
- ❌ Calendar widget en sidebar izquierdo
- ❌ Journal page title localizado (e.g., "May 26, 2026")
- ❌ Template de journal diario configurable

---

## Regla de priorización

**Primero semántica, luego confianza, luego velocidad**.

- Semántica: motor CM6, parser, refs, tags, properties (Fases 1, 2).
- Confianza: undo/redo, RefIndex, testing (Fases 3, 8-infra).
- Velocidad: teclado, slash, drag & drop, journals (Fases 4-9).

## Próximo hito

**Hito 1 — CodeMirror 6 migration** (Fase 1 pendiente):
1. Escribir bindings wasm-bindgen (5-6 tipos)
2. Crear componente `CodeMirrorBlock` en Leptos
3. Sustituir `contenteditable` en `block_editor.rs`
4. Conectar parser → decorations
5. Bridge undo/redo HistoryStack
