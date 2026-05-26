# ADR-0007: CodeMirror 6 como motor de edición por bloque

Status: accepted

Quilt usa CodeMirror 6 (CM6) como motor de edición textual dentro de cada bloque del outliner. CM6 se integra con Leptos 0.8 CSR via wasm-bindgen manual — no existe un crate Rust maduro para CM6, por lo que los bindings se escriben a mano (~200-300 líneas para EditorView, EditorState, StateEffect, Decoration, keymap). Una sola instancia CM6 se monta cuando un bloque recibe focus, y se desmonta al perderlo. En cualquier momento hay como máximo 1 instancia activa, idéntico al patrón que usa Logseq DB.

## Considered Options

1. **CodeMirror 6 con bindings wasm-bindgen manuales** — accepted: motor maduro, decorations nativas, extension system, WASM-compatible, cursor/selección/IME resueltos. No hay crate Rust pero los bindings son mecánicos. ~60KB gzipped para la configuración mínima (single-line, sin gutters).
2. **ProseMirror** — rejected: más flexible pero más complejo, sin patrón "block editor" establecido, comunidad más pequeña.
3. **contenteditable especializado** — rejected: reconstruir cursor, selección, IME, composición CJK, decorations desde cero es meses de trabajo y es exactamente el problema actual.
4. **CodeMirror 5 via crate `codemirror` (slowtec)** — rejected: es CM5 (legacy), abandonado (5 stars, 3 commits), y CM6 es la versión requerida.

## Consequences

- El componente Leptos `CodeMirrorBlock` usa `NodeRef<html::Div>` + `Effect::new` para montar CM6 imperativamente sobre un div. Patrón probado con `wasm-bindgen`.
- Los bindings manuales viven en `crates/quilt-ui/src/editor/codemirror.rs` (o módulo dedicado). Tipos: `EditorView`, `EditorState`, `StateEffect`, `Decoration`, `keymap`.
- CM6 se configura en modo single-line: sin line numbers, sin gutters, sin folding. Extensiones: custom keymap (Enter/Tab/Backspace → intents), decorations (refs, tags, properties), history (bridge al HistoryStack del Outliner).
- El parser inline (`InlineParser`) se conecta a CM6 via ViewPlugin que reacciona a `docChanged` y actualiza decorations.
- Undo/redo de texto dentro de un bloque se desactiva en CM6 (`history()` extension no se incluye). El historial lo maneja el Outliner via `HistoryStack`.
- Bundle size estimado: ~60KB gzipped para el subset de CM6 necesario.
