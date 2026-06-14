# ADR-0007: TipTap (ProseMirror) as per-block editor engine

Status: accepted

Quilt usa **TipTap** (framework basado en ProseMirror) como motor de edición de texto dentro de cada bloque del outliner. TipTap se integra nativamente con React 19 a través de `@tiptap/react`.

Inicialmente se consideró CodeMirror 6 (CM6), pero se optó por TipTap por las siguientes razones:

- **Rich text out of the box**: TipTap/ProseMirror ofrece bold, italic, código, headers, lists, y otras marcas inline sin necesidad de construir decorations manualmente via ViewPlugin
- **React integration nativa**: `@tiptap/react` proporciona hooks (`useEditor`) y componentes (`EditorContent`) que se integran directamente con el ciclo de vida de React, sin necesidad de bindings wasm-bindgen manuales (~200-300 líneas que CM6 requería)
- **Plugin ecosystem**: TipTap tiene extensiones para colaboración (`@tiptap/extension-collaboration`), placeholders, menús slash, autocomplete, y más
- **Flexibilidad**: ProseMirror permite control total sobre el documento y las transformaciones, mientras que TipTap abstrae la complejidad para casos de uso comunes

El parser inline (`InlineParser`) corre en WASM (quilt-core) y es independiente del motor de edición. TipTap maneja la edición de texto; el WASM parsea el contenido para resaltado semántico en modo lectura (page refs `[[ ]]`, block refs `(( ))`, tags, propiedades, etc.) a través del componente `InlineContent`.

## Considered Options

1. **TipTap (ProseMirror)** — accepted: rich text nativo, React integration, ecosistema de extensiones, comunidad activa
2. **CodeMirror 6** — rejected: requería bindings wasm-bindgen manuales, sin rich text nativo (solo texto plano con decorations), decorations complejas para resaltado semántico, integración React no nativa
3. **contentEditable especializado** — rejected: reconstruir cursor, selección, IME, composición CJK, decorations desde cero es meses de trabajo
4. **CodeMirror 5 via crate `codemirror` (slowtec)** — rejected: legacy, abandonado

## Consequences

- TipTap se instancia con `StarterKit` (bold, italic, code, history, headings, lists, etc.) y se separa en un chunk aparte via Vite `rollupOptions.output.manualChunks` (~150KB vs ~60KB de CM6)
- El componente de edición por bloque (`BlockRow`) usa `contentEditable` directamente con guards de teclado personalizados (Enter, Tab, Backspace, Arrow keys) que traducen a operaciones del outliner
- En modo lectura, `InlineContent` renderiza el contenido parseado por WASM con resaltado semántico (page refs, block refs, tags, properties, bold, italic, code)
- El parser inline (WASM) es independiente del editor — corre en `quilt-core` y se invoca desde React via `WasmProvider`
- Se mantienen dependencias de CodeMirror 6 (`@codemirror/view`, `@codemirror/state`) en `package.json` para funcionalidades auxiliares no relacionadas con edición de bloques (e.g., vistas de código)
- Undo/redo de bloque lo maneja el Outliner via `HistoryStack` a nivel de bloque, no a nivel de editor
- El bundle total del editor (React + TipTap + utilidades) es ~200KB gzipped, aceptable para una SPA CSR
