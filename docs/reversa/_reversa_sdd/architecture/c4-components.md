# C4 Components (C3) — Logseq

> **Escala**: 🟢 CONFIRMADO | 🟡 INFERIDO | 🔴 LACUNA
> **Nivel**: Componentes (descomposición interna de cada container)
> **Fecha**: 2026-05-02

---

## Diagrama de Componentes

```mermaid
C4Component
    accTitle: Logseq Component Architecture
    accDescr: Nivel C3 - Descomposición interna de containers principales

    Container_Boundary(frontend_renderer, "Electron Renderer") {
        Container_Boundary(components, "Components") {
            Component(container, "Root Container", "Rum", "App shell, routing, context menus")
            Component(editor, "Editor", "Rum", "Block editing, commands, auto-complete")
            Component(block, "Block", "Rum", "Block rendering, assets, timestamps")
            Component(page, "Page", "Rum", "Page title, blocks list, references")
            Component(journal, "Journal", "Rum", "Daily notes view")
            Component(query, "Query", "Rum", "DSL query execution, result rendering")
            Component(left_sidebar, "Left Sidebar", "Rum", "Navigation, favorites, recent")
            Component(right_sidebar, "Right Sidebar", "Rum", "Contextual panels, TOC")
            Component(header, "Header", "Rum", "Toolbar, search, actions")
            Component(settings, "Settings", "Rum", "Configuration UI")
            Component(property, "Property", "Rum", "Property dialog, config, value")
        }

        Container_Boundary(state_management, "State Management") {
            Component(state_atoms, "State Atoms", "Clojure", "Global UI state (route, sidebar, editor)")
            Component(event_bus, "Event Bus", "core.async", "Pub/Sub for event distribution")
            Component(db_react, "React Queries", "Rum", "Reactive DataScript queries")
        }
    }

    Container_Boundary(frontend_handler, "Event Handlers") {
        Component(events_loop, "Events Loop", "core.async", "Go-loop event processor")
        Component(handle_multimethod, "Handle (multimethod)", "Clojure", "Event dispatch by type")
        Component(editor_handler, "Editor Handler", "Clojure", "Save, insert, delete blocks")
        Component(page_handler, "Page Handler", "Clojure", "Create, rename, delete pages")
        Component(repo_handler, "Repo Handler", "Clojure", "Graph management")
        Component(ui_handler, "UI Handler", "Clojure", "Sidebar, theme, modals")
        Component(search_handler, "Search Handler", "Clojure", "Search index management")
    }

    Container_Boundary(datascript_layer, "DataScript Layer") {
        Component(conn, "Connection", "DataScript", "DB connection management")
        Component(transact, "Transact", "Clojure", "Async transaction pipeline")
        Component(query_dsl, "DSL Query", "Clojure", "Query string → Datalog parser")
        Component(query_react, "React Query", "Clojure", "Reactive query for UI")
        Component(model, "Model", "Clojure", "Domain functions (get-block, get-page)")
        Component(async_db, "Async DB", "Clojure", "Worker-thread DB access")
        Component(persist, "Persist", "Clojure", "IndexedDB/SQLite persistence")
    }

    Container_Boundary(graph_parser_layer, "Graph Parser") {
        Component(extract, "Extract", "Clojure", "Main extraction pipeline")
        Component(mldoc_wrapper, "Mldoc Wrapper", "Clojure", "Markdown/Org AST")
        Component(block_parser, "Block Parser", "Clojure", "Block-level parsing")
        Component(property_handler, "Property Handler", "Clojure", "Property extraction")
        Component(text_utils, "Text Utils", "Clojure", "Title parsing, refs")
    }

    Container_Boundary(outliner_layer, "Outliner") {
        Component(tree_ops, "Tree Operations", "Clojure", "Tree CRUD, move, indent")
        Component(op_pipeline, "Op Pipeline", "Clojure", "Operation hooks, transactions")
        Component(transaction, "Transaction", "DataScript", "Datalog transactions")
    }

    Rel(container, state_atoms, "Reads/Writes state")
    Rel(container, event_bus, "Publishes events")
    Rel(editor, state_atoms, "Editor state")
    Rel(editor, editor_handler, "Triggers events")
    Rel(block, db_react, "Reactive query")
    Rel(page, db_react, "Reactive query")
    Rel(journal, db_react, "Reactive query")
    Rel(query, query_dsl, "Executes query")
    Rel(query, db_react, "Reactive results")
    Rel(left_sidebar, event_bus, "Publishes nav events")
    Rel(right_sidebar, event_bus, "Publishes panel events")

    Rel(events_loop, handle_multimethod, "Dispatches")
    Rel(handle_multimethod, editor_handler, "Handles :editor/*")
    Rel(handle_multimethod, page_handler, "Handles :page/*")
    Rel(handle_multimethod, repo_handler, "Handles :graph/*")
    Rel(handle_multimethod, ui_handler, "Handles :ui/*")
    Rel(handle_multimethod, search_handler, "Handles :search/*")

    Rel(editor_handler, transact, "Writes data")
    Rel(page_handler, transact, "Writes data")
    Rel(editor_handler, outliner_layer, "Tree operations")
    Rel(transact, conn, "Transaction")

    Rel(query_dsl, model, "Domain functions")
    Rel(query_react, conn, "Query execution")
    Rel(db_react, query_react, "React wrapper")
    Rel(model, conn, "Entity access")
    Rel(async_db, persist, "Persistence")

    Rel(graph_parser_layer, transact, "Indexes content")
    Rel(extract, mldoc_wrapper, "Parses content")
    Rel(extract, block_parser, "Block parsing")
    Rel(extract, property_handler, "Props")
```

---

## Componentes por Container

---

## 1. Frontend Components 🟢

### 1.1 Container (Root)

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/components/container.cljs` |
| **Función** | `root-container`, `main`, `custom-context-menu` |
| **Responsabilidad** | App shell, routing, global context menus |

**Estado**:
```clojure
{:route-match      ; Current route
 :main-content     ; Active page/component
 :sidebar-open?    ; Left sidebar state
}
```

---

### 1.2 Editor 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/components/editor.cljs` |
| **Funciones** | `box`, `commands`, `search-pages`, `block-search`, `filter-commands`, `node-render` |
| **Responsabilidad** | Block editing, slash commands, page/block search |

**Comandos slash** (filtrados por contexto):
```clojure
/heading, /bullet, /numbered, /todo, /done
/priority, /deadline, /scheduled, /date
/code, /quote, /callout, /template
```

**Editor state**:
```clojure
{:editor/action       ; Current action
 :editor/cursor-range ; [start end]
 :editor/content      ; Current content
 :editor/block        ; Block being edited
}
```

---

### 1.3 Block 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/components/block.cljs` |
| **Funciones** | `page-cp`, `page-reference`, `page-inner`, `asset-container`, `resizable-image`, `timestamp` |
| **Responsabilidad** | Block rendering, assets, timestamps, references |

**Tipos de bloque**:
- Regular block (contenido)
- Page reference `[[page]]`
- Block reference `{{uuid}}`
- Asset (image, PDF, audio)
- Timestamp (scheduled, deadline)
- Query result

---

### 1.4 Page 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/components/page.cljs` |
| **Funciones** | `get-page-name`, `get-page-entity`, `page-cp`, `page-blocks-cp`, `db-page-title`, `global-graph` |
| **Responsabilidad** | Page view, title, blocks list |

**Block state**:
```clojure
{:db/id int
 :block/uuid uuid
 :block/name string
 :block/page {:db/id int :block/name string}
 :page? boolean
 :nlp-date? boolean}
```

---

### 1.5 Query 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/components/query.cljs` |
| **Sub-componentes** | `query/builder.cljs`, `query/result.cljs`, `query/view.cljs` |
| **Funciones** | `custom-query`, `custom-query-inner`, `query-title` |
| **Responsabilidad** | Query rendering, result grouping |

**Query DSL operators**:
```
(and or not) — Booleanos
(between x y) — Rangos
(property key value) — Propiedades
(task todo doing done) — Estados
(priority A B C) — Prioridades
(page name) — Página específica
(sample n) — Muestreo
```

---

## 2. Event Handlers 🟢

### 2.1 Events Loop 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/handler/events.cljs` |
| **Función** | `run!` |
| **Responsabilidad** | core.async go-loop para procesamiento de eventos |

```clojure
;; Event flow
(state/pub-event! [:event-type payload])
    → events channel
    → (go-loop [])
        → (handle payload)  ; multimethod dispatch
        → error handling + Sentry
```

---

### 2.2 Handle (multimethod) 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/handler/events.cljs` |
| **Función** | `handle` (defmulti) |
| **Responsabilidad** | Dispatch de eventos por tipo |

**Eventos principales**:

| Evento | Handler | Descripción |
|--------|---------|-------------|
| `:graph/switch` | `graph-switch-on-persisted` | Cambio de grafo |
| `:page/create` | `create!` | Nueva página |
| `:page/renamed` | `rename!` | Página renombrada |
| `:editor/save-current-block` | `save-current-block!` | Guardar bloque |
| `:editor/insert-block` | `insert-new-block!` | Insertar bloque |
| `:editor/delete-block` | `delete-block!` | Eliminar bloque |
| `:search/rebuild-index` | `schedule-search-index-build!` | Rebuild índice |

---

### 2.3 Editor Handler 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/handler/editor.cljs` |
| **Archivos relacionados** | `db_based/editor.cljs`, `common/editor.cljs` |
| **Responsabilidad** | Operaciones CRUD del editor |

**Operaciones**:
- `save-current-block!` — Persist current block
- `insert-new-block!` — Insert after current
- `delete-block!` — Remove selected blocks
- `edit-block!` — Start editing block
- `wrap-parse-block` — Parse and resolve refs

---

### 2.4 Page Handler 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/handler/page.cljs` |
| **Archivos relacionados** | `db_based/page.cljs`, `common/page.cljs` |
| **Responsabilidad** | Gestión de páginas |

**Operaciones**:
- `<create!` — Create page with tag validation
- `delete-repo!` — Delete graph
- `restore-and-setup-repo!` — Restore graph

---

## 3. DataScript Layer 🟢

### 3.1 Connection 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/db/conn.cljs` |
| **Funciones** | `get-db`, `transact!`, `start!` |
| **Responsabilidad** | DB connection lifecycle |

```clojure
;; Connection lifecycle
(start! repo opts) → conn  ; Initialize
(get-db repo deref?) → db  ; Access
(transact! repo tx-data) → void  ; Write
```

---

### 3.2 Transact 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/db/transact.cljs` |
| **Funciones** | `transact`, `apply-outliner-ops` |
| **Responsabilidad** | Async transaction pipeline |

```clojure
;; Transaction flow
(worker-transact repo tx-data tx-meta)
    → promise
    → [conn ops opts]
    → outliner pipeline
    → DataScript commit
```

---

### 3.3 DSL Query 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/db/query_dsl.cljs` |
| **Funciones** | `query`, `parse`, `build-query`, `custom-query` |
| **Responsabilidad** | Parse DSL string → Datalog |

**Pipeline de parsing**:
```clojure
"query string"
    → (pre-transform s)       ; Normalize
    → (parse s db opts)      ; Tokenize + build AST
    → (build-query e env)    ; Generate Datalog
    → DataScript query
```

---

### 3.4 Model 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/db/model.cljs` |
| **Funciones** | `get-block-by-uuid`, `get-page`, `get-journal-page`, `has-children?`, `get-next`, `get-prev` |
| **Responsabilidad** | Domain model accessors |

**Cardinalidad de funciones**:
- **Bloques**: 40+ funciones de acceso
- **Páginas**: 20+ funciones de acceso
- **Queries**: navegación, hijos, padres, refs

---

### 3.5 React Query 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `src/main/frontend/db/react.cljs`, `query_react.cljs` |
| **Funciones** | `q`, `react-query`, `refresh!`, `refresh-affected-queries!` |
| **Responsabilidad** | Reactive query system for UI |

**Cache key structure**:
```clojure
[q :key query-opts inputs*]
    → QueryCacheEntry
    → result-atom
    → Component tracking
```

---

## 4. Graph Parser 🟢

### 4.1 Extract 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `deps/graph-parser/src/logseq/graphql/extract.cljc` |
| **Función** | `extract` |
| **Responsabilidad** | Main pipeline: file → pages + blocks + AST |

**Pipeline**:
```
file-path + content
    → detect format (markdown/org)
    → mldoc/->edn → AST
    → extract-pages-and-blocks
    → [:pages :blocks :ast]
```

---

### 4.2 Block Parser 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `deps/graph-parser/src/logseq/graphql/block.cljs` |
| **Funciones** | `extract-blocks`, `construct-block`, `with-parent-and-order` |
| **Responsabilidad** | Parse AST → Block entities |

**Propiedades extraídas**:
- `block/title`
- `block/refs` (page refs `[[page]]`)
- `block.block-refs` (block refs `{{uuid}}`)
- `block/tags`
- Timestamps (created-at, updated-at)

---

## 5. Outliner 🟢

### 5.1 Tree Operations 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivos** | `deps/outliner/src/logseq/outliner/core.cljs`, `tree.cljs` |
| **Responsabilidad** | Block tree structure manipulation |

**Funciones de árbol**:
- `blocks->vec-tree` — Convert flat list to tree
- `block-entity->map` — Entity to map conversion
- `filter-top-level-blocks` — Root blocks only
- `non-consecutive-blocks->vec-tree` — Handle gaps

---

### 5.2 Operations 🟢

| Aspecto | Detalle |
|---------|---------|
| **Archivo** | `deps/outliner/src/logseq/outliner/op.cljs` |
| **Responsabilidad** | Operation definitions for tree mutations |

**Operations**:
```clojure
:save-block           ; Persist block
:insert-blocks       ; Insert new blocks
:delete-blocks       ; Remove blocks
:move-blocks         ; Move within/across pages
:indent-outdent-blocks ; Nesting change
```

---

## Dependencias entre componentes

```
┌──────────────────────────────────────────────────────────────┐
│                     Components (UI)                          │
│  container → [editor, block, page, journal, query]           │
│      ↓                                                          │
│  state_atoms ← event_bus                                     │
└──────────────────────────────────────────────────────────────┘
                              ↓ events
┌──────────────────────────────────────────────────────────────┐
│                     Event Handlers                            │
│  events_loop → handle_multimethod                            │
│      ↓                                                          │
│  [editor_handler, page_handler, repo_handler, ui_handler]     │
└──────────────────────────────────────────────────────────────┘
                              ↓ transact
┌──────────────────────────────────────────────────────────────┐
│                     DataScript Layer                         │
│  [conn, transact, query_dsl, model, react_queries]           │
└──────────────────────────────────────────────────────────────┘
                              ↑
┌──────────────────────────────────────────────────────────────┐
│  Graph Parser → [extract, mldoc_wrapper, block_parser]       │
│  Outliner → [tree_ops, op_pipeline]                         │
└──────────────────────────────────────────────────────────────┘
```

---

## Complejidad de componentes

| Componente | Archivos | Complejidad | Razón |
|------------|----------|-------------|-------|
| Editor | 1 | 🟢 Media | Lógica de comandos es lineal |
| Query | 4 | 🟡 Media | Parser DSL tiene recursión |
| Block | 1 | 🟢 Media | Render es straight-through |
| Model | 1 | 🟡 Media | 70+ funciones, bajo acoplamiento |
| Events Loop | 1 | 🟢 Baja | Go-loop simple |
| Transact | 1 | 🟡 Media | Async con promise chaining |
| Graph Parser | 10+ | 🟡 Media | Pipeline complejo pero lineal |
| Outliner | 15+ | 🟡 Alta | Operaciones de árbol con edge cases |

---

*Generado por Reversa Architect - 2026-05-02*
