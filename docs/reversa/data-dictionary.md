# Data Dictionary - Logseq

> Diccionario completo de datos del proyecto Quilt.
> Generado por: reversa-archaeologist
> Fecha: 2026-05-02
> Nivel de documentación: completo
>
> **⚠️ CANONICAL SCHEMA SOURCE**: Este documento describe el schema de **Logseq (ClojureScript original)**.
> Para el schema canonical de **Quilt (Rust)**, ver `docs/roadmap.md` Sección 1.3 y `docs/architecture-ddd.md`.
>
> **📌 CONVENCIÓN**: Si `erd.md` y este archivo divergen, **este archivo es canónico**.
> `erd.md` es un índice visual complementario; para detalles completos ver este diccionario.
>
> **Reconciliación**: El schema de Quilt/Rust es una reimplementación del schema de Logseq. Las diferencias
> principales están documentadas en `docs/reversa/rust-reimplementation-proposal.md`.

---

## Índice

1. [Entidades Core](#1-entidades-core)
2. [Entidades de Propiedades](#2-entidades-de-propiedades)
3. [Entidades de Referencias](#3-entidades-de-referencias)
4. [Entidades de Búsqueda](#4-entidades-de-búsqueda)
5. [Entidades de Sync/RTC](#5-entidades-de-syncrtc)
6. [Entidades de Plugins](#6-entidades-de-plugins)
7. [Enumeraciones y Constantes](#7-enumeraciones-y-constantes)

---

## 1. Entidades Core

### 1.1 Block

**Descripción**: Unidad fundamental de contenido en Logseq. Todo es un bloque.

| Campo | Tipo | Requerido | Descripción | Ejemplo |
|-------|------|-----------|-------------|---------|
| `:db/id` | `Int` | ✅ | Identificador interno de DataScript | `12345` |
| `:block/uuid` | `UUID` | ✅ | UUID público del bloque | `#uuid "550e8400-e29b-41d4-a716-446655440000"` |
| `:block/name` | `String` | ❌ | Nombre canónico (solo pages) | `"mi-pagina"` |
| `:block/title` | `String` | ❌ | Título visible | `"Mi primera nota"` |
| `:block/content` | `String` | ❌ | Contenido raw (deprecated) | - |
| `:block/format` | `Keyword` | ✅ | Formato del contenido | `:markdown`, `:org` |
| `:block/page` | `Ref` | ❌ | Página padre (si es bloque) | `{:db/id 123}` |
| `:block/parent` | `Ref` | ❌ | Bloque padre directo | `{:db/id 124}` |
| `:block/_parent` | `[Ref]` | ❌ | Hijos directos (reverse) | `[{...} {...}]` |
| `:block/order` | `String` | ❌ | Orden lexicográfico entre siblings | `"a0"` |
| `:block/level` | `Int` | ❌ | Nivel de indentación (1-indexed) | `1`, `2`, `3` |
| `:block/collapsed?` | `Boolean` | ❌ | Si está colapsado | `true`, `false` |
| `:block/tags` | `[Ref]` | ❌ | Tags/clases asociados | `[{:db/id 100}]` |
| `:block/refs` | `[Ref]` | ❌ | Referencias a otras páginas/bloques | `[page-ref block-ref]` |
| `:block/properties` | `Map` | ❌ | Propiedades key-value | `{:priority "A", :foo "bar"}` |
| `:block/properties-text-values` | `Map` | ❌ | Texto original de propiedades | `{:priority "A", :foo "bar"}` |
| `:block/properties-order` | `[Keyword]` | ❌ | Orden de propiedades | `[:id :priority :foo]` |
| `:block/invalid-properties` | `Set` | ❌ | Propiedades inválidas | `#{:unknown-prop}` |
| `:block/journal-day` | `Int` | ❌ | Día como entero (YYYYMMDD) | `20260502` |
| `:block/alias` | `[Map]` | ❌ | Alias de página | `[{:block/name "alias1" :block/title "Alias 1"}]` |
| `:block/namespace` | `Ref` | ❌ | Namespace de página | `{:db/id 200}` |
| `:block/file` | `Ref` | ❌ | Archivo de origen | `{:file/path "pages/foo.md"}` |
| `:block/type` | `String` | ❌ | Tipo especial | `"macro"`, `"journal"`, `"page"` |
| `:block/pre-block?` | `Boolean` | ❌ | Si es pre-block (propiedades) | `true` |
| `:block/created-at` | `Long` | ❌ | Timestamp de creación | `1717200000000` |
| `:block/updated-at` | `Long` | ❌ | Timestamp de actualización | `1717200000000` |
| `:block/unordered?` | `Boolean` | ❌ | Si es lista desordenada | `true` |
| `:block/heading` | `Boolean` | ❌ | Si es heading | `true` |

**Indexes**:
- `:block/uuid` (unique)
- `:block/name` (unique cuando presente)
- `:block/page`
- `:block/parent`

### 1.2 Page

**Descripción**: Representa una página en Logseq. Hereda de Block.

| Campo | Tipo | Requerido | Descripción | Ejemplo |
|-------|------|-----------|-------------|---------|
| `:db/id` | `Int` | ✅ | Identificador interno | `123` |
| `:block/uuid` | `UUID` | ✅ | UUID de la página | `#uuid "..."` |
| `:block/name` | `String` | ✅ | Nombre canónico lowercase | `"mi-pagina"` |
| `:block/title` | `String` | ❌ | Título original (case preserved) | `"Mi Página"` |
| `:block/format` | `Keyword` | ✅ | Formato default | `:markdown` |
| `:block/file` | `Ref` | ❌ | Archivo asociado | `{:file/path "pages/mi-pagina.md"}` |
| `:block/alias` | `[Map]` | ❌ | Alias/nombres alternativos | v. supra |
| `:block/tags` | `[Ref]` | ❌ | Tags de la página | `[TagA TagB]` |
| `:block/journal-day` | `Int` | ❌ | Día de journal si aplica | `20260502` |
| `:block/namespace` | `Ref` | ❌ | Namespace padre | `{:block/name "parent"}` |
| `:block/created-at` | `Long` | ❌ | Timestamp creación | `...` |
| `:block/updated-at` | `Long` | ❌ | Timestamp actualización | `...` |

**Cardinalidad**: Page es un Block con `:block/name` único.

### 1.3 File

**Descripción**: Representa un archivo en el filesystem.

| Campo | Tipo | Requerido | Descripción | Ejemplo |
|-------|------|-----------|-------------|---------|
| `:db/id` | `Int` | ✅ | Identificador interno | `1` |
| `:file/path` | `String` | ✅ | Ruta del archivo | `"journals/2026-05-02.md"` |
| `:file/content` | `String` | ❌ | Contenido en memoria | `"# Título..."` |

**Constraints**:
- `:file/path` es único

### 1.4 kv (Key-Value Store)

**Descripción**: Almacenamiento clave-valor para configuración.

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `:kv/key` | `Keyword` | ✅ | Clave | `:logseq.kv/latest-code-lang` |
| `:kv/value` | `Any` | ✅ | Valor | `"clojure"` |

---

## 2. Entidades de Propiedades

### 2.1 Property Schema

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `:db/ident` | `Keyword` | Identificador único |
| `:block/title` | `String` | Nombre mostrado |
| `:logseq.property/type` | `Keyword` | Tipo de dato |
| `:logseq.property/repeatable?` | `Boolean` | Si acepta múltiples valores |
| `:logseq.property/queryable?` | `Boolean` | Si es queryable |
| `:logseq.property/built-in?` | `Boolean` | Si es built-in |
| `:logseq.property/view-context` | `Keyword` | Contexto de visualización |
| `:logseq.property.default-value/value` | `Ref` | Valor por defecto |
| `:logseq.property.class/extends` | `Ref` | Clase que extiende |

### 2.2 Built-in Properties

| Property | Tipo | Descripción |
|----------|------|-------------|
| `title` | `String` | Título de la página |
| `alias` | `[String]` | Nombres alternativos |
| `tags` | `[PageRef]` | Tags/clasificaciones |
| `priority` | `String` | Prioridad (A/B/C) |
| `schedule` | `Timestamp` | Fecha programada |
| `deadline` | `Timestamp` | Fecha límite |
| `created` | `Timestamp` | Fecha de creación |
| `updated` | `Timestamp` | Fecha de actualización |
| `id` | `CustomID` | ID personalizado |
| `logseq.property/heading` | `Boolean` | Es heading |
| `logseq.property/node/display-type` | `Keyword` | Tipo de nodo (code, etc.) |
| `logseq.property.code/lang` | `Keyword` | Lenguaje de código |
| `logseq.macro-name` | `String` | Nombre de macro |
| `logseq.macro-arguments` | `[String]` | Args de macro |

### 2.3 Property Types

| Tipo | Descripción | Ejemplo |
|------|-------------|---------|
| `:db.type/string` | Texto | `"hello"` |
| `:db.type/ref` | Referencia a entidad | `{:block/name "page"}` |
| `:db.type/long` | Entero | `123` |
| `:db.type/boolean` | Booleano | `true` |
| `:node` | Número de página | `42.5` |

---

## 3. Entidades de Referencias

### 3.1 Page Reference

**Descripción**: Referencia a una página dentro de un bloque.

**Forma en AST (Mldoc)**:
```clojure
["Link" {:url ["Page_ref" "Page Name"] :label [["Plain" "Page Name"]]}]
```

**Forma en DB**:
```clojure
{:block/name "page-name"
 :block/title "Page Title"}
```

### 3.2 Block Reference

**Descripción**: Referencia a un bloque específico.

**Forma en AST**:
```clojure
["Block_reference" "uuid-string"]
```

**Forma en DB**:
```clojure
{:block/uuid #uuid "uuid-string"}
```

### 3.3 Tag (Class)

**Descripción**: Tags son páginas con clase Tag.

```clojure
{:db/ident :logseq.class/Tag
 :block/name "my-tag"
 :block/title "My Tag"
 :block/tags [...]}  ; tags pueden tener tags
```

### 3.4 Namespace

**Descripción**: Páginas que actúan como namespaces.

```clojure
{:block/name "parent/child"
 :block/namespace {:block/name "parent"}}
```

---

## 4. Entidades de Búsqueda

### 4.1 Query DSL

**Descripción**: Lenguaje de consultas de Logseq.

**Gramática**:
```clojure
;; Query simple
(query-string) => "(and [[Page]] (task now later))"

;; Operadores
(and query*)      ; Intersección
(or query*)       ; Unión
(not query)       ; Negación

;; Filtros
(between start end)         ; Rango de fechas
(property key value)        ; Propiedad específica
(task marker*)              ; Filtrar por estado
(priority level*)            ; Filtrar por prioridad
(page "Page Name")          ; Página específica
[[page-ref]]                ; Referencia directa
(full-text-search "text")   ; Búsqueda full-text
(sample n)                  ; Muestra aleatoria
```

### 4.2 Task Markers

| Marker | Significado | Estados |
|--------|-------------|---------|
| `NOW` | Tarea en progreso | Activo |
| `LATER` | Planificado | Pendiente |
| `TODO` | Por hacer | Pendiente |
| `DONE` | Completado | Completo |
| `CANCELLED` | Cancelado | Cancelado |

### 4.3 Search Index

**Estructura interna**:
```clojure
;; Blocks index
{:search/block-uuid UUID
 :search/content String
 :search/page-name String}

;; Pages index
{:search/page-name String
 :search/page-title String}
```

---

## 5. Entidades de Sync/RTC

### 5.1 RTC State

**Descripción**: Estado de sincronización en tiempo real.

```clojure
{:rtc/state
 {:editing-block-uuid UUID
  :client-id String
  :presence [{:client-id String
              :cursor {:block-uuid UUID
                       :offset Int}}]}}
```

### 5.2 Sync Operations

| Op | Descripción |
|----|-------------|
| `:save-block` | Guardar bloque |
| `:insert-blocks` | Insertar bloques |
| `:delete-blocks` | Eliminar bloques |
| `:move-blocks` | Mover bloques |
| `:move-blocks-up-down` | Mover arriba/abajo |
| `:indent-outdent-blocks` | Indentar/outdentar |
| `:transact` | Transacción raw |

### 5.3 Transaction Metadata

```clojure
{:db-sync/tx-id UUID          ; ID de transacción
 :local-tx? Boolean           ; Si es local
 :client-id String            ; Cliente origen
 :outliner-op Keyword         ; Tipo de operación
 :outliner-ops [...]          ; Ops semánticas}
```

---

## 6. Entidades de Plugins

### 6.1 Plugin Schema

```clojure
{:plugin/id String
 :plugin/name String
 :plugin/version String
 :plugin/api-version String
 :plugin/description String
 :plugin/sync? Boolean
 :plugin/settings Map}
```

### 6.2 Plugin Hooks

| Hook | Payload | Descripción |
|------|---------|-------------|
| `hook:db-tx` | `{:blocks :tx-data}` | Tx de base de datos |
| `hook:block-changes` | `{:blocks tx-data}` | Cambios en bloques |
| `search:rebuildPagesIndice` | `{}` | Rebuild índice páginas |
| `search:rebuildBlocksIndice` | `{}` | Rebuild índice bloques |

---

## 7. Enumeraciones y Constantes

### 7.1 Formatos

```clojure
:markdown  ; Formato Markdown
:org       ; Formato Org-mode
```

### 7.2 Bloques especiales

```clojure
:logseq.class/Journal     ; Página de journal
:logseq.class/Tag         ; Tag/clase
:logseq.class/Page       ; Página normal
:logseq.class/Property    ; Schema de propiedad
:logseq.class/Root        ; Raíz de clases
```

### 7.3 Built-in pages

```clojure
; Pages reservadas del sistema
"Contents"           ; Página de contenido
"logseq/custom.css"  ; CSS personalizado
"logseq/bak/"        ; Backups
```

### 7.4 Query Operators

```clojure
;; Datalog rules
:between
:property
:ref-property
:scalar-property
:task
:priority
:tags
:page
:page-ref
:self-ref
:block-content
:has-property
:has-private-simple-query-property
:private-ref-property
:private-scalar-property
```

### 7.5 View Contexts

```clojure
:all      ; Todas las vistas
:page     ; Contexto de página
:block    ; Contexto de bloque
:class    ; Contexto de clase
```

### 7.6 Order Keys (Lexicographic)

**Descripción**: Sistema de orden lexicográfico para sibling blocks.

```clojure
;; Ejemplo de generación
(db-order/gen-key "a" "b")  ; => "aN"
(db-order/gen-n-keys 3)     ; => ["a" "aN" "aO"]
```

### 7.7 Journal Day Format

**Descripción**: Entero que representa una fecha (YYYYMMDD).

```clojure
20260502  ; 2 de Mayo de 2026
```

### 7.8 UUID Types

| Tipo | Prefijo | Uso |
|------|---------|-----|
| Journal Page | `:journal-page-uuid` | Páginas de journal |
| Block | (default) | Bloques normales |
| Custom | - | UUID especificado por usuario |

---

## Esquema Completo (DataScript)

```clojure
;; Versión actual del schema
(def schema-version "1.0.0")

;; Entidades principales
(def schema
  {:block/uuid           {:db/unique :unique}
   :block/name           {:db/unique :unique}
   :file/path            {:db/unique :unique}
   
   :block/page           {:db/cardinality :one
                          :db/valueType :ref}
   :block/parent         {:db/cardinality :one
                          :db/valueType :ref}
   :block/_parent        {:db/cardinality :many
                          :db/valueType :ref}
   :block/refs           {:db/cardinality :many
                          :db/valueType :ref}
   :block/tags           {:db/cardinality :many
                          :db/valueType :ref}
   :block/_tags          {:db/cardinality :many
                          :db/valueType :ref}
   
   :logseq.property/_value {:db/cardinality :many
                           :db/valueType :ref}
   
   ;; Atributos de página
   :block/alias          {:db/cardinality :many}
   :block/namespace      {:db/valueType :ref}
   :block/file           {:db/valueType :ref}
   
   ;; Timestamps
   :block/created-at     {:db/valueType :long}
   :block/updated-at     {:db/valueType :long}
   
   ;; Journal
   :block/journal-day    {:db/valueType :long
                          :db/index true}
   
   ;; Properties
   :block/properties     {:db/valueType :db.type/ref
                          :db/is-component true}
   :block/properties-text-values {:db/valueType :db.type/ref
                                 :db/is-component true}
   
   ;; Search
   :search/block         {:db/valueType :db.type/ref
                          :db/is-component true}
   :search/page          {:db/valueType :db.type/ref
                          :db/is-component true}})
```

---

## Relaciones entre entidades

```
┌─────────────────────────────────────────────────────────────┐
│                         Page                                │
│  (block/name único, puede tener :block/namespace)          │
│         │                           │                       │
│         │ 1:N                       │ N:1                    │
│         ▼                           ▼                       │
│ ┌──────────────┐           ┌──────────────────┐            │
│ │    Block     │◄──────────│    :block/page   │            │
│ │  (contenido) │           └──────────────────┘            │
│ └──────────────┘                                             │
│         │                                                    │
│         │ N:M                                                │
│         ▼                                                   │
│ ┌──────────────┐     ┌──────────────┐                       │
│ │ Page/Block   │────►│    Block     │                       │
│ │   Refs       │     │   (target)   │                       │
│ └──────────────┘     └──────────────┘                       │
│                                                             │
│ ┌──────────────┐                                           │
│ │     Tag      │ (Page con :logseq.class/Tag)              │
│ └──────────────┘                                           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                         File                                │
│         │                                                   │
│         │ 1:1                                               │
│         ▼                                                   │
│ ┌──────────────┐                                            │
│ │    Page      │                                            │
│  (source)     │                                            │
└─────────────────────────────────────────────────────────────┘
```

---

## Validaciones

### Page Title
- No puede contener: `/`, `#`, `?`, `:`, `|`, `<`, `>`, `*`, `"`, `\`
- No puede ser vacío
- No puede ser solo números

### Block UUID
- No puede cambiar una vez creado
- Validado en `outliner-core/-save`

### Property Names
- Lowercase after keyword conversion
- `/` reemplazado por `-`
- Spaces reemplazados por `-`
- `_` reemplazado por `-`

---

*Documento generado automáticamente por Reversa Archaeologist*
