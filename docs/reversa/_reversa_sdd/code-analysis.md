# Code Analysis - Logseq

> Documento consolidado con el análisis de todos los módulos del proyecto Quilt.
> Generado por: reversa-archaeologist
> Fecha: 2026-05-02
> Nivel de documentación: completo

---

## Índice

1. [frontend/components](#1-frontendcomponents---ui-components)
2. [frontend/db](#2-frontenddb---datascript-models)
3. [frontend/handler](#3-frontendhandler---event-handlers)
4. [frontend/fs](#4-frontendfs---file-system)
5. [frontend/format](#5-frontendformat---parsers)
6. [frontend/search](#6-frontendsearch---búsqueda)
7. [graph-parser](#7-graph-parser---parser-de-grafos)
8. [outliner](#8-outliner---sistema-outliner)
9. [electron](#9-electron---desktop-app)

---

## 1. frontend/components - UI Components

### Descripción
Módulo que contiene todos los componentes de interfaz de usuario de Logseq. Utiliza Rum (fork de React) para la construcción de componentes.

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `editor.cljs` | Editor principal de bloques |
| `block.cljs` | Componente de bloque individual |
| `page.cljs` | Componente de página |
| `journal.cljs` | Componente de journal |
| `query.cljs` | Componente de consultas |
| `left_sidebar.cljs` | Barra lateral izquierda |
| `right_sidebar.cljs` | Barra lateral derecha |
| `header.cljs` | Encabezado de la aplicación |
| `settings.cljs` | Panel de configuraciones |

### Funciones principales

#### `frontend.components.editor`

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `filter-commands` | `page? commands` | `filtered-commands` | Filtra comandos según el contexto (página o bloque) |
| `node-render` | `block q {:keys [db-tag?]}` | `hiccup` | Renderiza un nodo de búsqueda con highlighting |
| `commands` | `id format` | `rum-element` | Auto-complete para comandos slash |
| `page-on-chosen-handler` | `embed? input id q pos format` | `fn` | Manejador de selección de página |
| `matched-pages-with-new-page` | `partial-matched-pages db-tag? q` | `pages-list` | Agrega opción de nueva página a resultados |
| `search-pages` | `q db-tag? set-matched-pages!` | `p/do!` | Búsqueda de páginas con soporte para tags |

#### `frontend.components.block`

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `block-unique-title` | `block opts` | `string` | Genera título único para bloques |
| `block-title-with-icon` | `block title icon` | `hiccup` | Renderiza título con icono |
| `breadcrumb` | `repo uuid opts` | `rum-element` | Breadcrumb de navegación |

### Entidades UI (estructuras de datos)

```clojure
;; Block component state
{:block/uuid uuid?
 :block/title string?
 :block/page {:db/id int :block/name string}
 :block/parent {:db/id int}
 :alias {:block/title string}
 :nlp-date? boolean
 :page? boolean}

;; Editor state
{:editor/cursor-range [start end]
 :block.editing/direction :up|:down|:max
 :block.editing/pos int}
```

### Dependencias del módulo
- `frontend.db` - Acceso a datos
- `frontend.handler.editor` - Manejadores de eventos del editor
- `frontend.state` - Estado global
- `rum.core` - Framework de componentes

---

## 2. frontend/db - DataScript Models

### Descripción
Módulo de acceso a datos que gestiona la conexión con DataScript y proporciona funciones de consulta y transacción.

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `model.cljs` | Funciones core de acceso a datos |
| `transact.cljs` | Transacciones asíncronas |
| `query_dsl.cljs` | Parser y ejecutor de queries DSL |
| `query_custom.cljs` | Queries personalizadas avanzadas |
| `conn.cljs` | Gestión de conexiones |
| `persist.cljs` | Persistencia de datos |
| `react.cljs` | React queries (reactivas) |
| `async.cljs` | Versiones async de funciones db |

### Funciones principales

#### `frontend.db.model`

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `get-block-by-uuid` | `id` | `entity` | Obtiene bloque por UUID |
| `query-block-by-uuid` | `id` | `entity` | Retorna bloque o página según UUID |
| `get-page` | `page-id-name-or-uuid` | `entity` | Obtiene página |
| `get-journal-page` | `page-name` | `entity` | Obtiene página de journal |
| `get-today-journal-page` | `[]` | `entity` | Obtiene journal del día actual |
| `get-latest-journals` | `n` | `entities` | Obtiene N journals más recientes |
| `page-exists?` | `page-name tags` | `boolean` | Verifica existencia de página |
| `has-children?` | `block-id` | `boolean` | Verifica si bloque tiene hijos |
| `get-next` | `db db-id opts` | `entity` | Obtiene siguiente bloque (navegación) |
| `get-prev` | `db db-id` | `entity` | Obtiene bloque anterior |
| `get-all-classes` | `repo opts` | `entities` | Obtiene todas las clases |
| `get-all-properties` | `graph opts` | `entities` | Obtiene todas las propiedades |
| `get-structured-children` | `repo eid` | `entities` | Hijos estructurados para clases |

#### `frontend.db.transact`

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `transact` | `worker-transact repo tx-data tx-meta` | `promise` | Ejecuta transacción async |
| `apply-outliner-ops` | `conn ops opts` | `promise` | Aplica operaciones del outliner |

#### `frontend.db.query-dsl`

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `query` | `repo query-string opts` | `results` | Ejecuta query DSL |
| `custom-query` | `repo query-m opts` | `results` | Ejecuta query custom (seq) |
| `parse` | `s db opts` | `parsed-map` | Parsea query string a datalog |
| `parse-query` | `q db opts` | `parsed-map` | Wrapper con template resolution |

### DSL Query Language (query_dsl.cljs)

```clojure
;; Operadores booleanos
(and)
(or)
(not)

;; Filtros
(between start end)
(property key value)
(task marker*)
(priority level*)
(page page-name)
(sample n)
(full-text-search "text")
[[page-ref]]

;; Time helpers
today, yesterday, tomorrow
-7d, +7d (días relativos)
-1w, +1w (semanas)
-1m, +1m (meses)
-1y, +1y (años)
-1h, +1h (horas)
-1n (minutos)
```

### Entidades DataScript

```clojure
;; Page/Block base
{:db/id int
 :block/uuid uuid
 :block/name string
 :block/title string
 :block/format :markdown|:org
 :block/page {:db/id int}
 :block/parent {:db/id int}
 :block/order string
 :block/level int
 :block/collapsed? boolean
 :block/tags [{:db/id int :block/name string}]
 :block/refs [{:db/id int :block/name string}]
 :block/journal-day int
 :block/created-at timestamp
 :block/updated-at timestamp}

;; File entity
{:file/path string
 :file/content string}

;; Journal
{:block/journal-day int
 :block/tags [:logseq.class/Journal]}
```

### Dependencias del módulo
- `datascript.core` - Base de datos
- `logseq.db` - Schema y funciones de base de datos
- `frontend.state` - Estado global
- `logseq.outliner.op` - Operaciones del outliner

---

## 3. frontend/handler - Event Handlers

### Descripción
Sistema de manejo de eventos centralizado. Utiliza `core.async` para el event loop y `multimethod` para dispatch de eventos.

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `events.cljs` | Event loop principal y definición de eventos |
| `editor.cljs` | Manejadores del editor |
| `page.cljs` | Manejadores de páginas |
| `repo.cljs` | Manejadores de repositorios |
| `ui.cljs` | Manejadores de UI |
| `search.cljs` | Manejadores de búsqueda |
| `graph.cljs` | Manejadores de grafos |
| `export.cljs` | Manejadores de exportación |

### Sistema de eventos (events.cljs)

```clojure
;; Event dispatch
(state/pub-event! [:event-name payload])

;; Event handler pattern (multimethod)
(defmulti handle first)

(defmethod handle :event/name [[_ payload]]
  ;; handler logic
  )
```

### Eventos definidos

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `:graph/switch` | `graph opts` | Cambio de grafo |
| `:graph/open-new-window` | `target-repo` | Abrir nueva ventana |
| `:page/create` | `page-name opts` | Crear página |
| `:page/deleted` | `page-name tx-meta` | Página eliminada |
| `:page/renamed` | `repo data` | Página renombrada |
| `:graph/ready` | `repo` | Grafos listo para mostrar |
| `:graph/restored` | `graph` | Grafos restaurados (window reload) |
| `:graph/sync-context` | `-` | Sincronizar contexto al worker |
| `:editor/set-heading` | `block heading` | Establecer heading |
| `:editor/quick-capture` | `args` | Captura rápida |
| `:editor/save-current-block` | `-` | Guardar bloque actual |
| `:editor/upsert-type-block` | `{:keys [block type lang]}` | Cambiar tipo de bloque |
| `:db/sync-changes` | `data` | Cambios sincronizados de DB |
| `:db/export-sqlite` | `-` | Exportar a SQLite |
| `:graph/save-db-to-disk` | `opts` | Guardar DB a disco |
| `:rtc/sync-state` | `state` | Estado de sincronización RTC |
| `:rtc/download-remote-graph` | `graph-name uuid schema e2ee?` | Descargar grafo remoto |

### Flujo de eventos

```
User Action → state/pub-event! → events channel → handle multimethod → Handler Function
                                                              ↓
                                                    db/transact o efectos
                                                              ↓
                                                    pipeline/invoke-hooks
```

### Funciones principales

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `run!` | `[]` | `chan` | Inicia el event loop |
| `<build-search-index!` | `repo` | `promise` | Construye índice de búsqueda |
| `schedule-search-index-build!` | `repo` | `-` | Programa rebuild del índice |

### Dependencias del módulo
- `frontend.db` - Transacciones de base de datos
- `frontend.db.model` - Consulta de modelos
- `frontend.handler.editor` - Lógica del editor
- `frontend.handler.page` - Lógica de páginas
- `clojure.core.async` - Event loop

---

## 4. frontend/fs - File System

### Descripción
Abstracción del sistema de archivos que soporta múltiples implementaciones (local, cloud, etc.).

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `protocol.cljs` | Definición del protocolo Fs |
| `node.cljs` | Implementación para Node.js (Electron) |
| `memory_fs.cljs` | Implementación en memoria |

### Protocolo Fs

```clojure
(defprotocol Fs
  (mkdir! [this dir])
  (mkdir-recur! [this dir])
  (readdir [this dir])
  (unlink! [this repo path opts])
  (rmdir! [this dir])
  (read-file [this dir path opts])
  (read-file-raw [this dir path opts])
  (write-file! [this repo dir path content opts])
  (rename! [this repo old-path new-path])
  (copy! [this repo old-path new-path])
  (stat [this path])
  (open-dir [this dir])
  (get-files [this dir])
  (watch-dir! [this dir options])
  (unwatch-dir! [this dir]))
```

### Métodos del protocolo

| Método | Retorno | Descripción |
|--------|---------|-------------|
| `mkdir!` | `any` | Crea directorio |
| `mkdir-recur!` | `any` | Crea directorio recursivamente |
| `readdir` | `[string]` | Lista archivos en directorio |
| `unlink!` | `any` | Elimina archivo |
| `rmdir!` | `any` | Elimina directorio |
| `read-file` | `string` | Lee archivo como string |
| `read-file-raw` | `bytes` | Lee archivo como bytes |
| `write-file!` | `any` | Escribe archivo |
| `rename!` | `any` | Renombra archivo |
| `copy!` | `any` | Copia archivo |
| `stat` | `{:type string :size number :mtime number}` | Estadísticas de archivo |
| `open-dir` | `{:path string :files [...]}` | Abre directorio (nuevo grafo) |
| `get-files` | `[{:path string :content string}]` | Obtiene archivos |
| `watch-dir!` | `any` | Observa directorio |
| `unwatch-dir!` | `any` | Deja de observar |

### Dependencias del módulo
- `fs-extra` (Node.js)
- Protocolo definido en `frontend.fs.protocol`

---

## 5. frontend/format - Parsers

### Descripción
Módulo de parsing que maneja Markdown y Org-mode mediante Mldoc (librería externa).

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `protocol.cljs` | Protocolo de formato |
| `mldoc.cljs` | Wrapper de Mldoc |
| `block.cljs` | Funciones de parsing de bloques |

### Protocolo de formato

```clojure
(defprotocol Format
  (toEdn [_this content config])
  (toHtml [_this content config references])
  (exportMarkdown [_this content config references])
  (exportOPML [_this content config title references]))
```

### Funciones principales

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `parse-export-markdown` | `content config references` | `string` | Parsea y exporta a markdown |
| `parse-export-opml` | `content config title references` | `string` | Parsea y exporta a OPML |
| `->edn` | `content format` | `ast` | Convierte contenido a EDN |
| `plain->text` | `plains` | `string` | Extrae texto plano |
| `extract-first-query-from-ast` | `ast` | `query-string` | Extrae primera query de AST |

### Formatos soportados
- `:markdown` / `:md` - Markdown
- `:org` - Org-mode

### Mldoc AST (ejemplo simplificado)

```clojure
;; Heading
["Heading" {:size 1 :anchor "anchor"} [["Plain" "Title"]]]

;; Paragraph
["Paragraph" [["Plain" "text"]]]

;; Property drawer
["Drawer" "properties" [...]]

;; Code block
["Code" {:language "clojure"} "code content"]

;; Block reference
["Block_reference" "uuid"]
```

### Dependencias del módulo
- `mldoc` - Librería de parsing externa
- `frontend.format.protocol` - Definición de protocolo

---

## 6. frontend/search - Búsqueda

### Descripción
Sistema de búsqueda que soporta múltiples motores (browser native, plugins).

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `protocol.cljs` | Protocolo de motor de búsqueda |
| `agency.cljs` | Agencia que coordina motores |
| `browser.cljs` | Implementación nativa del browser |
| `plugin.cljs` | Implementación via plugin API |

### Protocolo de búsqueda

```clojure
(defprotocol Engine
  (query [_this q opts])
  (rebuild-blocks-indice! [_this])
  (rebuild-pages-indice! [_this])
  (transact-blocks! [_this data])
  (truncate-blocks! [_this])
  (remove-db! [_this]))
```

### Estructura de Agency

```clojure
;; Agency coordina múltiples motores de búsqueda
;; 1. Browser engine (siempre presente)
;; 2. Plugin engines (si LSP habilitado)

(get-registered-engines repo)
;; => [Browser (Plugin1) (Plugin2) ...]

;; Query fluye a todos los motores
(query [_this q opts]
  (doseq [e all-engines]
    (protocol/query e q opts)))
```

### Funciones principales

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `query` | `repo q opts` | `results` | Ejecuta búsqueda en todos los motores |
| `rebuild-blocks-indice!` | `repo` | `-` | Rebuild índice de bloques |
| `rebuild-pages-indice!` | `repo` | `-` | Rebuild índice de páginas |
| `transact-blocks!` | `repo data` | `-` | Transacciona cambios de bloques |
| `truncate-blocks!` | `repo` | `-` | Limpia índice de bloques |

### Dependencias del módulo
- `frontend.search.protocol` - Protocolo de motor
- `frontend.state` - Estado global
- Plugin API para motores externos

---

## 7. graph-parser - Parser de Grafos

### Descripción
Librería externa (deps/) que maneja el parsing de archivos Markdown/Org a estructuras de datos para DataScript.

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `extract.cljc` | Extracción de páginas y bloques |
| `block.cljs` | Parsing de bloques individuales |
| `mldoc.cljc` | Wrapper de Mldoc |
| `property.cljs` | Manejo de propiedades |
| `text.cljs` | Utilidades de texto |

### Función principal: `extract`

```clojure
(defn extract
  "Extracts pages, blocks and ast from given file"
  [file-path content {:keys [user-config verbose] :as options}]
  => {:pages [...blocks]
      :blocks [...pages]
      :ast ast})
```

### Pipeline de extracción

```
1. file-path + content
        ↓
2. Detect format (markdown/org) via common-util/get-format
        ↓
3. mldoc/->edn → AST
        ↓
4. extract-pages-and-blocks
   a. get-page-name (title parsing)
   b. extract-blocks (from AST)
   c. build-page-map (properties)
   d. build-pages-aux (namespaces)
        ↓
5. [pages blocks ast]
```

### Funciones principales

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `extract` | `file-path content options` | `{:pages :blocks :ast}` | Extrae todo de un archivo |
| `extract-pages-and-blocks` | `format ast props file content opts` | `[pages blocks]` | Extrae páginas y bloques |
| `title-parsing` | `file-name-body filename-format` | `string` | Parsea nombre de archivo a título |
| `build-page-map` | `props invalid file page page-name opts` | `page-map` | Construye mapa de página |
| `with-ref-pages` | `pages blocks` | `pages-with-refs` | Agrega páginas referenciadas |

### block.cljs - Parsing de bloques

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `extract-blocks` | `ast content format options` | `blocks` | Extrae bloques del AST |
| `heading-block?` | `block` | `boolean` | Verifica si es heading |
| `get-page-reference` | `block format` | `page-ref` | Extrae referencia de página |
| `get-block-reference` | `block` | `block-ref` | Extrae referencia de bloque |
| `extract-properties` | `props user-config` | `{:properties :page-refs :block-refs}` | Extrae propiedades |
| `construct-block` | `block props timestamps body enc format pos-meta opts` | `block-map` | Construye mapa de bloque |
| `fix-block-id-if-duplicated!` | `db page-name *extracted-block-ids block` | `block` | Corrige IDs duplicados |
| `with-parent-and-order` | `page-id blocks` | `blocks` | Agrega parent y order |

### Lógica no-trivial

#### Título de página
```clojure
;; Orden de detección de título:
1. property title:: (propiedad en primer bloque)
2. file name parsing (nombre del archivo)
3. first heading content (contenido del primer heading)
```

#### Journal detection
```clojure
;; Un archivo es journal si:
;; - El nombre matchea con patrón de fecha (configurable)
;; - Convertido via convert-page-if-journal
```

### Dependencias del módulo
- `mldoc` - Parsing de markdown/org
- `datascript.core` - Acceso a DB
- `logseq.db` - Funciones de base de datos
- `logseq.common.util` - Utilidades comunes

---

## 8. outliner - Sistema Outliner

### Descripción
Core del sistema outliner que maneja operaciones CRUD sobre bloques jerárquicos.

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `core.cljs` | Operaciones principales |
| `tree.cljs` | Manipulación de árboles |
| `op.cljs` | Transacción de operaciones |
| `transaction.cljc` | Transacción de Datascript |
| `op/construct.cljc` | Construcción de operaciones |
| `pipeline.cljs` | Pipeline de procesamiento |

### Protocolo INode

```clojure
(defprotocol INode
  (-save [this *txs-state conn opts])
  (-del [this *txs-state db]))
```

### Operaciones del outliner

| Operación | Args | Descripción |
|-----------|------|-------------|
| `:save-block` | `[block opts]` | Guardar bloque |
| `:insert-blocks` | `[blocks target-block opts]` | Insertar bloques |
| `:delete-blocks` | `[blocks opts]` | Eliminar bloques |
| `:move-blocks` | `[blocks target-block opts]` | Mover bloques |
| `:move-blocks-up-down` | `[blocks up?]` | Mover arriba/abajo |
| `:indent-outdent-blocks` | `[blocks indent? opts]` | Indentar/outdentar |

### Funciones principales

#### core.cljs

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `save-block` | `db block opts` | `{:tx-data}` | Guarda bloque |
| `insert-blocks` | `db blocks target-block opts` | `{:tx-data :blocks}` | Inserta bloques |
| `delete-blocks` | `db blocks opts` | `{:tx-data}` | Elimina bloques |
| `move-blocks` | `conn blocks target-block opts` | `-` | Mueve bloques |
| `move-blocks-up-down` | `conn blocks up?` | `-` | Mueve arriba/abajo |
| `indent-outdent-blocks` | `conn blocks indent?` | `-` | Indenta/outdenta |
| `tree-vec-flatten` | `tree-vec` | `blocks` | Convierte árbol a lista |
| `blocks-with-level` | `blocks` | `blocks` | Calcula niveles |

#### tree.cljs

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `blocks->vec-tree` | `db blocks root-id` | `tree` | Convierte bloques a árbol |
| `block-entity->map` | `e` | `map` | Convierte entidad a mapa |
| `filter-top-level-blocks` | `blocks` | `blocks` | Filtra bloques de nivel superior |
| `non-consecutive-blocks->vec-tree` | `blocks` | `tree` | Árbol con bloques no consecutivos |

### Algoritmos no-trivial

#### Inserción de bloques
```clojure
;; 1. Calcular niveles de bloques
;; 2. Determinar parent target (sibling vs child)
;; 3. Generar orders únicos
;; 4. Build insert tx
;; 5. Handle template refs
```

#### Movimiento de bloques
```clojure
;; 1. Validar movimiento (no circular)
;; 2. Calcular nuevo parent y order
;; 3. Mover children si cambia página
;; 4. Handle propiedad created-from
```

#### Delete con orphan handling
```clojure
;; 1. Identificar top-level blocks
;; 2. Verificar consecutividad
;; 3. Para páginas huérfanas → recycle
;; 4. Para refs huérfanos → cleanup
```

### Dependencias del módulo
- `datascript.core` - Base de datos
- `logseq.db` - Schema y funciones
- `logseq.db.common.order` - Generación de órdenes
- `malli.core` - Validación de schemas

---

## 9. electron - Desktop App

### Descripción
Módulo principal de Electron para la aplicación desktop.

### Archivos principales

| Archivo | Propósito |
|---------|-----------|
| `core.cljs` | Entry point y setup |
| `window.cljs` | Gestión de ventanas |
| `handler.cljs` | IPC handlers |
| `db.cljs` | Base de datos local |
| `server.cljs` | Servidor HTTP interno |
| `updater.cljs` | Auto-update |

### Entry point

```javascript
// static/electron.js
// Electron main process entry
```

### Clases de window

```clojure
;; electron.window
(create-main-window!) → BrowserWindow
(setup-window-listeners! win) → cleanup-fn
(close-handler win e)
(switch-to-window! win)
```

### IPC Handlers

```clojure
;; electron.handler
(set-ipc-handler! win)
(stop-all-db-workers!)
```

### Protocols personalizados

```clojure
;; Custom protocols
lsp://          - Logseq protocol
assets://       - Asset serving
```

### Funciones principales

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `main` | `[]` | `-` | Entry point principal |
| `setup-interceptor!` | `app` | `cleanup-fn` | Registra file protocols |
| `on-app-ready!` | `app` | `-` | Handler de app ready |
| `install-cli-launcher!` | `[]` | `-` | Instala CLI launcher |

### Flujo de inicialización

```
1. requestSingleInstanceLock
2. registerSchemesAsPrivileged
3. registerDefaultProtocolClient
4. set-app-menu!
5. setup-deeplink!
6. on-app-ready!
   a. setup-interceptor!
   b. create-main-window
   c. setup-updater
   d. setup-app-manager
   e. set-ipc-handler
   f. server/setup
7. window events listeners
```

### Dependencias del módulo
- `electron` - Framework
- `fs-extra` - Sistema de archivos
- `electron-log` - Logging

---

## Resumen de dependencias entre módulos

```
┌─────────────────────────────────────────────────────────┐
│                     electron                            │
│  (Electron main process, window management)            │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────┴───────────────────────────────────┐
│                   frontend                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │components│  │ handler  │  │    db    │  │   fs   │ │
│  │  (UI)    │──│ (events) │──│ (datascript)│ │(files)│ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
│       │            │              │             │      │
│       │            │              │             │      │
│  ┌────┴────────────┴──────────────┴─────────────┴────┐ │
│  │                  format                            │ │
│  │              (mldoc parsing)                       │ │
│  └────────────────────────┬───────────────────────────┘ │
│                           │                             │
│  ┌────────────────────────┴───────────────────────────┐ │
│  │                    search                           │ │
│  │              (agencia + engines)                   │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                      │
┌─────────────────────┴───────────────────────────────────┐
│                   deps/                                  │
│  ┌──────────────────┐  ┌───────────────────────────┐   │
│  │  graph-parser    │  │        outliner           │   │
│  │  (file → db)     │  │    (tree operations)      │   │
│  └──────────────────┘  └───────────────────────────┘   │
│           │                        │                     │
│           └────────────────────────┴─────────────────────┘
│                              │
                    ┌──────────┴──────────┐
                    │     logseq.db      │
                    │  (datascript schema)│
                    └─────────────────────┘
```

---

## Métricas de complejidad

| Módulo | Archivos | Líneas (est.) | Complejidad |
|--------|----------|---------------|------------|
| components | 70+ | ~15000 | Alta |
| db | 15+ | ~3000 | Media |
| handler | 50+ | ~10000 | Alta |
| fs | 5+ | ~1000 | Baja |
| format | 5+ | ~500 | Baja |
| search | 5+ | ~500 | Baja |
| graph-parser | 10+ | ~3000 | Media |
| outliner | 15+ | ~4000 | Alta |
| electron | 25+ | ~5000 | Media |

---

## Entidades principales del sistema

### Bloque (Block)
```clojure
{:db/id Int
 :block/uuid UUID
 :block/name String
 :block/title String
 :block/content String
 :block/format (:markdown | :org)
 :block/page Ref
 :block/parent Ref
 :block/order String
 :block/level Int
 :block/collapsed? Boolean
 :block/tags [Ref]
 :block/refs [Ref]
 :block/properties Map
 :block/journal-day Int?
 :block/created-at Long
 :block/updated-at Long}
```

### Página (Page)
```clojure
{:db/id Int
 :block/uuid UUID
 :block/name String
 :block/title String
 :block/file Ref
 :block/alias [{:block/name String :block/title String}]
 :block/tags [Ref]
 :block/journal-day Int?
 :block/namespace Ref?
 :block/created-at Long
 :block/updated-at Long}
```

### Archivo (File)
```clojure
{:db/id Int
 :file/path String
 :file/content String}
```

---

## Notas de implementación

### Performance considerations
- Queries reactivas via `frontend.db.react`
- Búsqueda indexada en worker threads
- Lazy loading de bloques anidados
- Persistencia incremental

### Concurrencia
- Event loop via `core.async`
- Transacciones optimistas
- Conflict resolution para sync

### Testing
- `clojure.test` para unit tests
- Playwright para E2E

---

*Documento generado automáticamente por Reversa Archaeologist*
