# Arquitectura del Outliner Profesional — Quilt v2

> Status: Propuesta de diseño
> Fecha: 2026-05-25
> Basado en: ADR-0006, outliner-professional-baseline.md, logseq-ui-reference.md

---

## 1. Principios Fundamentales

### 1.1 El Outliner es la fuente de verdad, no el editor

```
┌─────────────────────────────────────────────────────────────┐
│                      DOMINIO (Outliner)                     │
│  Graph → Page → Block → Property + Ref + Tag                │
│  ← Fuente de verdad canónica                                │
└─────────────────────────────────────────────────────────────┘
                              ↑
                              │ Operaciones del Outliner
                              │ (no operaciones de texto)
┌─────────────────────────────────────────────────────────────┐
│              MOTOR DE EDICIÓN (adaptador por bloque)       │
│  Cursor, selección, IME, decoración visual, autocompletado  │
│  ← Detalles de interacción, NO estado persistente          │
└─────────────────────────────────────────────────────────────┘
```

**Regla de hierro**: Toda acción que cambie estructura, refs, propiedades o semántica del Block se resuelve como operación del Outliner. El motor de edición solo traduce inputs del usuario a intents.

### 1.2 Modelo de operación

- **Por-Bloque**: Cada Bloque tiene su propio editor textual (textarea o contenteditable minimal)
- **Coordinado a nivel de Página**: Las operaciones estructurales (indent, split, move) se coordinan desde Page
- **Parser unificado incremental**: Un solo parser por Bloque para `[[Page]]`, `((Block))`, `#tag`, `property:: value`

---

## 2. Componentes Mayúsculas

### 2.1 Diagrama de módulos

```
quilt-ui/
├── outliner/
│   ├── mod.rs                 # Punto de entrada público
│   ├── page.rs               # PageOutliner — coordinador de página
│   ├── block.rs              # BlockOutliner — operaciones por bloque
│   ├── ops.rs                # OutlinerOps — operaciones estructurales
│   ├── history.rs            # UndoRedo — historia de intenciones
│   └── tree.rs               # TreeOps — operaciones de árbol ( YA EXISTE )
│
├── parser/
│   ├── mod.rs                # Punto de entrada
│   ├── inline.rs             # Parser incremental para sintaxis inline
│   ├── property.rs           # Parser de propiedades tipadas
│   ├── autocomplete.rs       # Servicio de autocompletado
│   └── normalize.rs           # Normalización refs/tags → dominio
│
├── editor/
│   ├── mod.rs                # EditorFactory — crea editores por tipo
│   ├── text.rs               # TextEditor — editor textual por bloque
│   ├── cursor.rs             # CursorManager — gestión de cursor/selección
│   ├── input.rs              # InputHandler — captura de teclado
│   └── decorations.rs        # DecorationManager — decoraciones visuales
│
├── components/
│   ├── block_editor.rs      # YA EXISTE — refactorizar a editor/text.rs
│   ├── block.rs             # YA EXISTE — refactorizar a outliner/block.rs
│   └── keyboard_handlers.rs # YA EXISTE — refactorizar a editor/input.rs
│
└── state/
    ├── mod.rs                # OutlinerState — estado reactivo
    ├── selection.rs         # BlockSelection — selección múltiple
    └── editing.rs            # EditingState — qué bloque está editando
```

### 2.2 Responsabilidades por módulo

| Módulo | Responsabilidad | No responsabilidad |
|--------|-----------------|-------------------|
| **outliner/page** | Coordinar operaciones estructurales de la página | Cursor, selección, input |
| **outliner/block** | traducir intent de edición a operación del dominio | Layout visual, cursor |
| **outliner/ops** | Implementar indent, outdent, split, merge, move, collapse | Estado de UI |
| **outliner/history** | Cmd-Z/Cmd-Shift-Z como historia de intenciones | Deshacer texto del editor |
| **parser/inline** | Parsear `[[]]`, `(())`, `#`, `property::` de forma incremental | Persistencia, queries |
| **editor/text** | Rendering del texto, cursor, selección, IME | Operaciones estructurales |
| **editor/input** | Captura de teclado, mapping a intents | Persistencia |

---

## 3. Flujo de Datos y Eventos

### 3.1 Flujo de un keystroke

```
Usuario pulsa "Enter" en un bloque
         │
         ▼
┌──────────────────────────────────────┐
│ editor/input.rs                      │
│   InputHandler::dispatch(key, mods)  │
│   → Clasifica como Enter estándar   │
└──────────────────────────────────────┘
         │
         ▼ ( keystroke_event::Enter )
         │
┌──────────────────────────────────────┐
│ editor/text.rs                       │
│   TextEditor::handle_enter()         │
│   → Determina cursor = posición 42  │
│   → Calcula split point             │
└──────────────────────────────────────┘
         │
         ▼ ( intent::SplitBlock { cursor: 42 } )
         │
┌──────────────────────────────────────┐
│ outliner/block.rs                    │
│   BlockOutliner::split_at_cursor()   │
│   → NO modifica estado todavía       │
│   → Emite OutlinerEvent              │
└──────────────────────────────────────┘
         │
         ▼ ( outliner_event::Split { block_id, cursor } )
         │
┌──────────────────────────────────────┐
│ outliner/page.rs                     │
│   PageOutliner::transact(ops)        │
│   → Valida operación                │
│   → Muta estado del dominio          │
│   → Registra en history             │
│   → Notifica a UI via signal        │
└──────────────────────────────────────┘
         │
         ▼ ( state change → leptos signal )
         │
┌──────────────────────────────────────┐
│ Components (re-render)              │
│   Block components leen nuevo estado  │
└──────────────────────────────────────┘
```

### 3.2 Eventos del Outliner

```rust
// outliner/events.rs

/// Eventos que el motor de edición emite hacia el Outliner
pub enum OutlinerIntent {
    // Edición textual
    TextChanged { block_id: BlockId, new_text: String },
    CursorMoved { block_id: BlockId, offset: u32 },

    // Operaciones estructurales
    EnterPressed { block_id: BlockId, cursor: u32 },
    TabPressed { block_id: BlockId },
    ShiftTabPressed { block_id: BlockId },
    BackspaceOnEmpty { block_id: BlockId },

    // Operaciones de rango
    SelectionChanged { block_ids: Vec<BlockId> },
    DeleteRequested { block_ids: Vec<BlockId> },
}

/// Eventos que el Outliner emite hacia la UI
#[derive(Clone, Reactive)]
pub enum OutlinerEvent {
    BlockCreated { block: Block },
    BlockDeleted { block_id: BlockId },
    BlockMoved { block_id: BlockId, new_parent: Option<BlockId>, new_order: f64 },
    BlockContentChanged { block_id: BlockId, content: String },
    BlockPropertyChanged { block_id: BlockId, property: Property },
    BlockCollapsedChanged { block_id: BlockId, collapsed: bool },
    PageChanged { page_id: PageId },
}
```

### 3.3 Modelo de estado reactivo

```rust
// outliner/state.rs

#[derive(Clone, Reactive)]
pub struct OutlinerState {
    /// Página actualmente abierta
    pub current_page: Signal<PageId>,

    /// Bloques de la página actual (fuente de verdad para render)
    pub blocks: Signal<Vec<Block>>,

    /// Qué bloque está siendo editado (None = ninguno)
    pub editing_block: Signal<Option<BlockId>>,

    /// Bloques seleccionados (para operaciones en masa)
    pub selected_blocks: Signal<Vec<BlockId>>,

    /// Historial de operaciones (para undo/redo)
    history: HistoryStack,

    /// Cola de eventos pendientes del motor de edición
    pending_intents: Vec<OutlinerIntent>,
}
```

---

## 4. Operaciones del Dominio (OutlinerOps)

### 4.1 Firma de operaciones

```rust
// outliner/ops.rs

/// Operaciones del Outliner — todas regresan Result y mutan estado
pub trait OutlinerOperations {
    // Creación / Destrucción
    fn create_block(&mut self, page_id: PageId, parent_id: Option<BlockId>, content: &str, order: f64) -> Result<Block, OutlinerError>;
    fn delete_block(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;

    // Estructurales
    fn indent(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;
    fn outdent(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;
    fn split(&mut self, block_id: BlockId, cursor: u32) -> Result<(Block, Block), OutlinerError>;
    fn merge_with_next(&mut self, block_id: BlockId) -> Result<Block, OutlinerError>;
    fn merge_with_previous(&mut self, block_id: BlockId) -> Result<Block, OutlinerError>;

    // Reordenamiento
    fn move_block(&mut self, block_id: BlockId, new_parent: Option<BlockId>, new_order: f64) -> Result<(), OutlinerError>;
    fn move_up(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;
    fn move_down(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;

    // Collapse
    fn toggle_collapse(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;
    fn expand(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;
    fn collapse(&mut self, block_id: BlockId) -> Result<(), OutlinerError>;

    // Propiedades (inline)
    fn set_property(&mut self, block_id: BlockId, property: Property) -> Result<(), OutlinerError>;
    fn remove_property(&mut self, block_id: BlockId, key: &str) -> Result<(), OutlinerError>;
    fn cycle_status(&mut self, block_id: BlockId) -> Result<Status, OutlinerError>;

    // Refs
    fn add_ref(&mut self, block_id: BlockId, target: RefTarget) -> Result<(), OutlinerError>;
    fn remove_ref(&mut self, block_id: BlockId, target: RefTarget) -> Result<(), OutlinerError>;

    // Tags
    fn add_tag(&mut self, block_id: BlockId, tag: TagName) -> Result<(), OutlinerError>;
    fn remove_tag(&mut self, block_id: BlockId, tag: TagName) -> Result<(), OutlinerError>;
}
```

### 4.2 Validaciones

Cada operación valida antes de ejecutarse:

| Operación | Validación |
|-----------|-----------|
| `indent` | El bloque tiene sibling anterior |
| `outdent` | El bloque tiene padre (no es root) |
| `move_block` | No se movería el bloque a sus propios descendientes (circular) |
| `merge_with_next` | Existe un sibling siguiente |
| `delete_block` | Si tiene hijos, deben moverse al padre primero |
| `add_ref` | El target existe y no es el propio bloque |

---

## 5. Frontera Parser / Editor / Outliner

### 5.1 Responsabilidades claras

```
┌──────────────────────────────────────────────────────────────────┐
│                         PARSER (parser/)                          │
│                                                                   │
│  ParserInline                                                    │
│  ├── parse(content: &str) -> ParsedContent                       │
│  ├── incremental(prev: &ParsedContent, delta: &str) -> Patch    │
│  └── normalize(parsed: ParsedContent) -> DomainEntities          │
│                                                                   │
│  Responsabilidades:                                              │
│  - Reconoce [[page]], ((block)), #tag, property:: value         │
│  - Devuelve estructura parsed con rangos de caracteres           │
│  - Normaliza a entidades del dominio (Ref, Tag, Property)       │
│  - NO sabe nada de cursor, selección, undo                       │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ ParsedContent + ranges
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                         EDITOR (editor/)                          │
│                                                                   │
│  TextEditor                                                      │
│  ├── render(parsed: &ParsedContent) -> View                      │
│  ├── handle_input(event: InputEvent) -> Option<Intent>           │
│  ├── get_cursor() -> CursorState                                │
│  ├── set_cursor(offset: u32)                                     │
│  └── apply_decorations(decos: Vec<Decoration>)                   │
│                                                                   │
│  Responsabilidades:                                              │
│  - Gestiona cursor y selección                                    │
│  - Rendering visual con decoraciones                              │
│  - Captura de input del usuario                                  │
│  - Detección de triggers para autocompletado (/ [[ (( #        │
│  - NO conoce operaciones estructurales                           │
│  - NO mantiene estado del dominio                                │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ Intent (enum)
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                        OUTLINER (outliner/)                      │
│                                                                   │
│  PageOutliner + BlockOutliner                                    │
│  ├── receive_intent(intent: Intent) -> Result<()>                │
│  ├── transact(ops: Vec<Op>) -> Result<Vec<Event>>               │
│  └── history.undo() / history.redo()                             │
│                                                                   │
│  Responsabilidades:                                               │
│  - Coordina operaciones estructurales                            │
│  - Mantiene historial de intenciones                             │
│  - Sincroniza con persistencia (backend via bridge)              │
│  - Emite eventos de vuelta a la UI                               │
└──────────────────────────────────────────────────────────────────┘
```

### 5.2 Parser Inline detallado

```rust
// parser/inline.rs

/// Contenido parseado con información de rangos
pub struct ParsedContent {
    pub raw_text: String,
    pub segments: Vec<Segment>,
}

pub enum Segment {
    Text { content: String, range: Range },
    PageRef { page_name: String, raw: String, range: Range },
    BlockRef { block_uuid: Uuid, raw: String, range: Range },
    Tag { name: String, raw: String, range: Range },
    Property { key: String, value: String, raw: String, range: Range },
}

/// Parser incremental que puede continuar desde un estado previo
pub struct InlineParser {
    state: ParserState,
}

impl InlineParser {
    /// Parse completo desde cero
    pub fn parse(&self, content: &str) -> ParsedContent;

    /// Parse incremental — usa state previo para eficiencia
    pub fn incremental(&mut self, content: &str) -> Patch;

    /// Normaliza a entidades del dominio
    pub fn normalize(&self, parsed: &ParsedContent) -> NormalizedContent {
        NormalizedContent {
            page_refs: parsed.page_refs().map(|r| PageRef::new(r.page_name)),
            block_refs: parsed.block_refs().map(|r| BlockRef::new(r.block_uuid)),
            tags: parsed.tags().map(|t| Tag::new(t.name)),
            properties: parsed.properties().map(|p| Property::new(p.key, p.value)),
        }
    }
}
```

### 5.3 DecorationManager

```rust
// editor/decorations.rs

/// Decoración visual para un rango de texto
pub struct Decoration {
    pub range: Range,
    pub kind: DecorationKind,
}

pub enum DecorationKind {
    PageLink { page_name: String },
    BlockLink { block_uuid: Uuid },
    Tag { tag_name: String },
    Property { key: String },
    SearchMatch { query: String },
    AutocompleteActive { index: usize },
}

/// Convierte ParsedContent en decoraciones visuales
pub struct DecorationManager;

impl DecorationManager {
    pub fn build_decorations(parsed: &ParsedContent) -> Vec<Decoration> {
        // Cada Segment genera DecorationKind apropiado
    }

    pub fn apply_to_editor(&self, editor: &mut TextEditor, decos: Vec<Decoration>);
}
```

---

## 6. Modelo de Undo/Redo

### 6.1 Principio: Historia de intenciones del Outliner

```
Logseq (referencia):
- Usa transact! que envuelve operaciones del outliner
- Cada transacción es un "comando" con undo/redo
- El historial es por página

Quilt v2 (implementación):
- Mismo modelo: HistoryStack por PageOutliner
- Cada operación estructural es un comando atómico
- El motor de edición NO mantiene historial propio
```

### 6.2 Estructura del History

```rust
// outliner/history.rs

/// Un comando en el historial
#[derive(Clone)]
pub enum OutlinerCommand {
    // Text operations
    SetContent { block_id: BlockId, prev: String, next: String },

    // Structural operations
    CreateBlock { block: Block },
    DeleteBlock { block: Block, prev_parent: Option<BlockId>, prev_order: f64 },
    MoveBlock { block_id: BlockId, from_parent: Option<BlockId>, from_order: f64, to_parent: Option<BlockId>, to_order: f64 },
    Indent { block_id: BlockId, prev_parent: Option<BlockId>, prev_order: f64 },
    Outdent { block_id: BlockId, prev_parent: Option<BlockId>, prev_order: f64 },
    Split { original: Block, new: Block },
    Merge { prev: Block, merged: Block },

    // Property operations
    SetProperty { block_id: BlockId, property: Property },
    RemoveProperty { block_id: BlockId, property: Property },

    // Ref operations
    AddRef { block_id: BlockId, ref: RefTarget },
    RemoveRef { block_id: BlockId, ref: RefTarget },

    // Marker/Status
    CycleStatus { block_id: BlockId, prev: Status, next: Status },
}

pub struct HistoryStack {
    commands: Vec<OutlinerCommand>,
    position: usize,  // índice actual en commands
}

impl HistoryStack {
    pub fn execute(&mut self, cmd: OutlinerCommand) {
        // Ejecuta el comando y lo agrega al historial
        self.commands.truncate(self.position);
        self.commands.push(cmd);
        self.position = self.commands.len();
    }

    pub fn undo(&mut self) -> Option<OutlinerCommand> {
        // Devuelve el comando anterior y retrocede position
    }

    pub fn redo(&mut self) -> Option<OutlinerCommand> {
        // Avanza position y devuelve el comando
    }
}
```

### 6.3 Integración con PageOutliner

```rust
impl PageOutliner {
    /// Ejecuta una operación y la registra en historial
    fn transact<F>(&mut self, op: F) -> Result<Vec<OutlinerEvent>, OutlinerError>
    where
        F: FnOnce(&mut Self) -> Result<Vec<OutlinerEvent>, OutlinerError>
    {
        let events = op(self)?;

        // Crear comando de undo basado en la operación
        let cmd = self.build_command(&events);
        self.history.execute(cmd);

        Ok(events)
    }

    /// Undo: invierte el último comando
    pub fn undo(&mut self) -> Result<Vec<OutlinerEvent>, OutlinerError> {
        let cmd = self.history.undo().ok_or(OutlinerError::NoMoreUndos)?;
        self.apply_inverse(cmd)
    }

    /// Redo: reaplica el último comando deshecho
    pub fn redo(&mut self) -> Result<Vec<OutlinerEvent>, OutlinerError> {
        let cmd = self.history.redo().ok_or(OutlinerError::NoMoreRedos)?;
        self.apply_forward(cmd)
    }
}
```

### 6.4 Commands compuestos

Para operaciones que generan múltiples cambios (ej: delete con hijos), se agrupan en un solo comando:

```rust
// Un solo comando para eliminar bloque + mover hijos al padre
OutlinerCommand::DeleteBlock {
    block: Block,           // bloque eliminado
    children_moved: Vec<(BlockId, Option<BlockId>, f64)>,  // hijos reubicados
    prev_parent: Option<BlockId>,
    prev_order: f64,
}
```

---

## 7. Key Interactions (Logseq-compatibles)

### 7.1 Tabla de bindings

| Acción | Keystroke | Intent generado | Operación del Outliner |
|--------|------------|-----------------|----------------------|
| Nuevo sibling | `Enter` | `Intent::Enter` | `split(block, cursor)` |
| Nueva línea | `Shift+Enter` | `Intent::SoftBreak` | `insert \n en texto` |
| Indent | `Tab` | `Intent::Tab` | `indent(block)` |
| Outdent | `Shift+Tab` | `Intent::ShiftTab` | `outdent(block)` |
| Borrar vacío | `Backspace` en cursor=0 | `Intent::DeleteEmpty` | `merge_with_previous` o `delete` |
| Cycle status | `Mod+Enter` | `Intent::CycleStatus` | `cycle_status(block)` |
| Collapse | `Mod+;` | `Intent::ToggleCollapse` | `toggle_collapse(block)` |
| Expand | `Mod+Down` | `Intent::Expand` | `expand(block)` |
| Collapse | `Mod+Up` | `Intent::Collapse` | `collapse(block)` |
| Move up | `Mod+Shift+Up` | `Intent::MoveUp` | `move_up(block)` |
| Move down | `Mod+Shift+Down` | `Intent::MoveDown` | `move_down(block)` |
| Undo | `Mod+Z` | — | `outliner.undo()` |
| Redo | `Mod+Shift+Z` | — | `outliner.redo()` |

### 7.2 Split en cursor

```
Contenido: "Hola [[Mundo]] cruel"
Cursor: posición 5 (dentro de "Hola ")

Split:
  → Bloque 1: "Hola "
  → Bloque 2: "[[Mundo]] cruel"

Los refs se mantienen: el parser los detecta y la split es limpia.
```

### 7.3 Merge de bloques

```
Bloque 1: "Hola "
Bloque 2: "Mundo cruel"

Merge:
  → Bloque 1: "Hola Mundo cruel"
  → Bloque 2: eliminado

El parser normaliza los refs del contenido combinado.
```

---

## 8. Puntos de Extensión

### 8.1 Plugin del Parser

```rust
/// Extensión para agregar sintaxis custom
pub trait ParserExtension {
    fn name(&self) -> &str;
    fn trigger(&self) -> char;  // ej: '/' para slash commands
    fn parse(&self, context: &str) -> Vec<ExtensionSegment>;
}
```

### 8.2 Plugin de Decoration

```rust
/// Extensión para decoraciones custom
pub trait DecorationExtension {
    fn name(&self) -> &str;
    fn decorate(&self, segment: &Segment) -> Option<Decoration>;
}
```

### 8.3 Slash Commands

```rust
/// Definición de un slash command
pub struct SlashCommand {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub category: String,
    pub execute: Arc<dyn Fn(&mut PageOutliner, BlockId)>,
}
```

### 8.4 Tipos de Editor

```rust
/// Factory para crear editores según el tipo de bloque
pub trait EditorFactory: Send + Sync {
    fn create_text_editor(&self, block: &Block) -> Box<dyn TextEditor>;
    fn create_code_editor(&self, block: &Block) -> Box<dyn CodeEditor>;
    // Fácil de extender para: math, quote, etc.
}
```

---

## 9. Estrategia de Migración

### 9.1 Estado actual

El código actual tiene:
- `contenteditable` en `BlockEditor` con manejo basic de keyboard
- `tree.rs` con `indent`, `outdent`, `split_block`, `merge_with_next`
- `Block` component que mezcla lógica de UI y de dominio

### 9.2 Fases de migración

**Fase 1: Parser + Decorations** (sin cambiar estructura)
```
- Crear parser/inline.rs con parser incremental
- Crear editor/decorations.rs
- Mantener BlockEditor con contenteditable
- El parser solo se usa para detectar [[]], (()), #, property::
```

**Fase 2: Intents + Commands**
```
- Definir OutlinerIntent enum
- Agregar editor/input.rs con InputHandler
- Modificar BlockEditor para emitir intents en vez de modificar estado directamente
- Crear outliner/history.rs
```

**Fase 3: Outliner State**
```
- Crear outliner/state.rs con OutlinerState
- PageOutliner coordina
- BlockOutliner traduce intents a operaciones
- Ya no se modifica blocks directamente desde componentes
```

**Fase 4: TextEditor refactor**
```
- Reemplazar contenteditable con editor/text.rs
- CursorManager para cursor/selección
- DecorationManager conectado al parser
```

**Fase 5: APIs de autocompletado**
```
- AutocompleteProvider trait
- Implementaciones para [[]], (()), #, /
- UI de dropdown
```

### 9.3 Archivos a crear/modificar

| Archivo | Acción | Razón |
|---------|--------|-------|
| `parser/inline.rs` | Crear | Parser incremental unificado |
| `parser/mod.rs` | Crear | Punto de entrada del parser |
| `editor/text.rs` | Crear | TextEditor (reemplaza contenteditable) |
| `editor/input.rs` | Crear | InputHandler (reemplaza keyboard_handlers) |
| `editor/decorations.rs` | Crear | DecorationManager |
| `editor/cursor.rs` | Crear | CursorManager |
| `outliner/page.rs` | Crear | PageOutliner |
| `outliner/block.rs` | Crear | BlockOutliner |
| `outliner/ops.rs` | Crear | OutlinerOperations trait |
| `outliner/history.rs` | Crear | HistoryStack |
| `outliner/events.rs` | Crear | Intent y Event enums |
| `outliner/state.rs` | Crear | OutlinerState |
| `components/block_editor.rs` | Refactorizar | → `editor/text.rs` + `editor/input.rs` |
| `components/block.rs` | Refactorizar | → `outliner/block.rs` |
| `components/keyboard_handlers.rs` | Deprecar | → `editor/input.rs` |
| `outliner/tree.rs` | Mantener | Lógica de árbol ya existe |

### 9.4 Convivencia durante migración

Durante las fases 1-4, el código viejo y nuevo coexisten:
- Feature flags para switching
- Tests paralelos: viejo y nuevo
- Para пользователь: sin cambios visibles hasta fase 4

---

## 10. Consideraciones Adicionales

### 10.1 Performance

- **Parser incremental**: El parseo completo en cada keystroke es costoso. El parser debe ser incremental (solo re-parsear lo que cambió)
- **Virtualización**: Para páginas con 1000+ bloques, renderizar solo los visibles
- **Señales granulares**: Leptos signals permiten re-render fino por bloque sin re-renderizar toda la página

### 10.2 IME (Input Method Editor)

- El editor debe ignorar keystrokes durante composición IME
- `is_composing` flag en InputHandler
- El commit de IME se trata como un `TextChanged` normal

### 10.3 Autocompletado

Triggers:
- `[[` → Page autocomplete
- `((` → Block autocomplete
- `#` → Tag autocomplete
- `/` → Slash command palette

El autocomplete es del editor, no del outliner. El outliner solo proporciona los datos (lista de páginas, bloques, tags).

### 10.4 Drag & Drop

Pendiente para fase post-baseline. Requiere:
- `DragState` en OutlinerState
- `drop_target` y `drop_position` (before/after/child)
- Visual drop indicator

---

## 11. Testing Strategy

### 11.1 Unit tests

- `parser/inline.rs`: Tests de parseo para cada tipo de inline
- `outliner/ops.rs`: Tests de cada operación con validaciones
- `outliner/history.rs`: Tests de undo/redo

### 11.2 Integration tests

- Flujo completo: keystroke → intent → operación → estado
- Probar todas las interacciones de Logseq (Tabla 7.1)

### 11.3 Snapshot tests

- Render de bloques con distintas decoraciones
- Proteger contra regresiones visuales

---

## Resumen de Convenciones de Nomenclatura

| Concepto | Nombre sugerido |
|----------|----------------|
| Editor por bloque | `TextEditor` |
| Coordinador de página | `PageOutliner` |
| Traductor de intents | `BlockOutliner` |
| Eventos del motor → outliner | `OutlinerIntent` |
| Eventos del outliner → UI | `OutlinerEvent` |
| Comandos de undo | `OutlinerCommand` |
| Parser de inline | `InlineParser` |
| Manager de decoraciones | `DecorationManager` |
| Manejador de input | `InputHandler` |
| Estado reactivo global | `OutlinerState` |

---

## Decisiones Abiertas

1. **TextEditor backend**: ¿textarea minimal, contenteditable especializado, o CodeMirror? Recomendación: contenteditable especializado (CodeMirror añade weight innecesario)

2. **Persistencia**: ¿El OutlinerState hace bridge calls directamente o hay un intermediario? Recomendación: puente claro via `Bridge` trait para poder mockear en tests

3. **Drag & Drop**: ¿Implementar en fase 1 o dejar para después? Recomendación: después — es complejo y el baseline funciona sin él

4. **Sync de edición**: ¿Cómo maneja múltiples editores abiertos? Recomendación: modo single-editor por simplicity initially, multi-editor post-baseline
