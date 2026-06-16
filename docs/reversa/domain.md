# Domain Glossary - Logseq

> Glosario de dominio y reglas de negocio extraídas del código y arqueología Git.
> Generado por: reversa-detective
> Fecha: 2026-05-02
Proyecto: Quilt
> Nivel: completo

---

## 1. Entidades Core

### 1.1 Block (Bloque)

**Definición**: Unidad fundamental de contenido en Logseq. Todo es un bloque.

**Atributos principales**:
- `:block/uuid` - Identificador único universal
- `:block/name` - Nombre canónico (solo para páginas)
- `:block/title` - Título visible
- `:block/content` - Contenido raw
- `:block/format` - Formato (`:markdown` | `:org`)
- `:block/page` - Página padre (referencia)
- `:block/parent` - Bloque padre directo
- `:block/order` - Orden lexicográfico entre siblings
- `:block/level` - Nivel de indentación (1-indexed)
- `:block/collapsed?` - Si está colapsado
- `:block/refs` - Referencias a otras páginas/bloques
- `:block/properties` - Mapa de propiedades key-value
- `:block/type` - Tipo especial (`"macro"`, `"journal"`, `"page"`)
- `:block/created-at` / `:block/updated-at` - Timestamps

**Reglas**:
- 🔴 **LACUNA**: El UUID no puede cambiar una vez creado (validado en `outliner-core/-save`)
- 🟢 **CONFIRMADO**: Los bloques heredan el formato de su página padre

---

### 1.2 Page (Página)

**Definición**: Representa una página en Logseq. Hereda de Block con `:block/name` único.

**Propiedades especiales**:
- `:block/alias` - Nombres alternativos
- `:block/tags` - Tags/clasificaciones
- `:block/namespace` - Namespace padre
- `:block/journal-day` - Día de journal (YYYYMMDD)

**Jerarquía**:
```
Namespace (parent)
  └── Page (child with namespace)
        └── Block
```

**Reglas**:
- 🟢 **CONFIRMADO**: Los nombres de página son case-insensitive (lowercase)
- 🟡 **INFERIDO**: Namespaces permiten organizar páginas jerárquicamente

---

### 1.3 Journal (Diario)

**Definición**: Página especial con fecha como nombre.

**Naming convention**:
- Formato: `YYYY-MM-DD` (configurable)
- Ejemplo: `2026-05-02`

**Reglas**:
- 🟢 **CONFIRMADO**: Los journals tienen día ordinal (`:block/journal-day` como entero YYYYMMDD)
- 🟢 **CONFIRMADO**: Los journals son páginas especiales con clase `:logseq.class/Journal`

---

### 1.4 Tag

**Definición**: Página con la clase `:logseq.class/Tag`.

**Características**:
- 🟢 **CONFIRMADO**: Tags son páginas referenciables con `#tag-name`
- 🟢 **CONFIRMADO**: Tags pueden tener otros tags (jerárquicos)

---

### 1.5 Property (Propiedad)

**Definición**: Metadatos key-value asociados a bloques y páginas.

**Built-in Properties**:

| Property | Tipo | Descripción |
|----------|------|-------------|
| `title` | String | Título de la página |
| `alias` | [String] | Nombres alternativos |
| `tags` | [PageRef] | Tags/clasificaciones |
| `priority` | String | Prioridad (A/B/C) |
| `schedule` | Timestamp | Fecha programada |
| `deadline` | Timestamp | Fecha límite |
| `created` | Timestamp | Fecha de creación |
| `updated` | Timestamp | Fecha de actualización |
| `id` | CustomID | ID personalizado |
| `publishing-public?` | Boolean | Visibilidad en publicación |

**Property Types**:
- `:db.type/string` - Texto
- `:db.type/ref` - Referencia a entidad
- `:db.type/long` - Entero
- `:db.type/boolean` - Booleano

**Reglas**:
- 🟢 **CONFIRMADO**: Los nombres de propiedad se normalizan a lowercase
- 🟢 **CONFIRMADO**: `/` en nombres se reemplaza por `-`
- 🟢 **CONFIRMADO**: Spaces se reemplazan por `-`
- 🟡 **INFERIDO**: Propiedades protegidas no pueden eliminarse (ej: `:block/name`)

---

## 2. Task System (Sistema de Tareas)

### 2.1 Task Markers

> **📌 Canonicalización**: Los task markers se almacenan en lowercase en persistencia.
> La forma canónica es: `now`, `todo`, `doing`, `done`, `later`, `waiting`, `cancelled`.
> El `MarkdownCanonicalizer` deriva la forma canónica desde el marker lowercase (ADR-0025 slice #3).

**Estados válidos** (forma canónica en storage):

| Marker | Keyword | Significado | Categoría |
|--------|---------|-------------|-----------|
| `now` | `:logseq.property/status.doing` | En progreso | Activo |
| `later` | `:logseq.property/status.later` | Planificado | Pendiente |
| `todo` | `:logseq.property/status.todo` | Por hacer | Pendiente |
| `doing` | `:logseq.property/status.doing` | En progreso (alias) | Activo |
| `done` | `:logseq.property/status.done` | Completado | Completo |
| `cancelled` | `:logseq.property/status.canceled` | Cancelado | Cancelado |
| `waiting` | `:logseq.property/status.waiting` | Bloqueado | Pendiente |

**Reglas**:
- 🟢 **CONFIRMADO**: Los markers son valores cerrados (closed values) del schema
- 🟢 **CONFIRMADO**: Storage usa lowercase (`now`, `todo`, `done`, etc.)
- 🟡 **INFERIDO**: Solo un marker activo por bloque a la vez
- 🟡 **INFERIDO**: `done` y `cancelled` son estados terminales

### 2.2 Priority System

**Niveles**:

| Priority | Keyword | Significado |
|----------|---------|-------------|
| `A` | `:logseq.priority/a` | Alta |
| `B` | `:logseq.priority/b` | Media |
| `C` | `:logseq.priority/c` | Baja |

---

## 3. Sistema de Archivos

### 3.1 Graph (Grafo)

**Definición**: Conjunto de páginas y archivos Markdown/Org que forman un repositorio de conocimiento.

**Estructura típica**:
```
graph-name/
├── journals/
│   ├── 2026-05-02.md
│   └── 2026-05-01.md
├── pages/
│   ├── my-page.md
│   └── another-page.md
├── logseq/
│   └── config.edn
└── logseq.db
```

**Reglas**:
- 🟢 **CONFIRMADO**: Cada graph tiene su propia base de datos DataScript
- 🟢 **CONFIRMADO**: Los graphs se sincronizan con servidor remoto (opcional)

---

### 3.2 File System Protocol

**Implementaciones**:

| Implementación | Uso |
|----------------|-----|
| `FsNode` | Node.js / Electron |
| `FsMemory` | Testing / Browser memory |

**Operaciones del protocolo**:

```clojure
(ls [this dir])                    ; Lista archivos
(mkdir! [this dir])                ; Crea directorio
(mkdir-recur! [this dir])         ; Crea directorio recursivo
(read-file [this dir path opts])   ; Lee archivo
(write-file! [this repo dir path content opts]) ; Escribe archivo
(rename! [this repo old-path new-path]) ; Renombra
(unlink! [this repo path opts])    ; Elimina archivo
(stat [this path])                 ; Estadísticas
(watch-dir! [this dir options])   ; Observa directorio
```

---

## 4. Sistema de Sincronización

### 4.1 Sync States

**Estados del sync**:

| Estado | Descripción |
|--------|-------------|
| `syncing` | Sincronización en progreso |
| `synced` | Sincronizado correctamente |
| `error` | Error de sincronización |
| `offline` | Sin conexión |

**Componentes del estado**:
- `:local-tx` - Transacción local pendiente
- `:remote-tx` - Última transacción remota
- `:checksum` - Checksum para validación
- `:e2ee?` - Encryption habilitado

### 4.2 Encryption (E2EE)

**Concepto**: End-to-end encryption para graphs remotos.

**Reglas**:
- 🟢 **CONFIRMADO**: E2EE es una funcionalidad pagada ("paid feature")
- 🟢 **CONFIRMADO**: Requiere contraseña para habilitación
- 🟢 **CONFIRMADO**: Los títulos de bloques se cifran manteniendo referencias

**Flujo**:
1. Usuario habilita sync con contraseña
2. Se genera clave de cifrado derivada de la contraseña
3. Contenido se cifra localmente antes de subir
4. Servidor solo almacena contenido cifrado

---

## 5. Sistema de Publicación

### 5.1 Publishing Model

**Concepto**: Publicar páginas seleccionadas como sitio web estático.

**Propiedades de publicación**:

| Property | Tipo | Descripción |
|----------|------|-------------|
| `publishing-public?` | Boolean | Visibilidad de la página |
| `publishing-slug` | String | URL slug personalizada |
| `publishing-url` | String | URL personalizada |
| `publishing-sitemap?` | Boolean | Incluir en sitemap |

**Modos de publicación**:
- 🟢 **CONFIRMADO**: `all-pages-public?` - Todas las páginas son públicas por defecto
- 🟢 **CONFIRMADO**: Por página (`publishing-public?`) - Visibilidad granular

### 5.2 Protected Pages

**Concepto**: Páginas protegidas con contraseña.

**Reglas**:
- 🟢 **CONFIRMADO**: Páginas pueden tener contraseña individual
- 🟢 **CONFIRMADO**: Contraseña diferente de la de E2EE
- 🟡 **INFERIDO**: Solo se solicita contraseña para ver, no para sincronizar

---

## 6. Sistema de Plugins

### 6.1 Plugin Hooks

**Hooks disponibles**:

| Hook | Payload | Descripción |
|------|---------|-------------|
| `hook:db-tx` | `{:blocks :tx-data}` | Tx de base de datos |
| `hook:block-changes` | `{:blocks tx-data}` | Cambios en bloques |
| `search:rebuildPagesIndice` | `{}` | Rebuild índice páginas |
| `search:rebuildBlocksIndice` | `{}` | Rebuild índice bloques |

### 6.2 Plugin API

**Capabilities**:
- 🟢 **CONFIRMADO**: Custom block renderers
- 🟡 **INFERIDO**: Themes de UI
- 🟡 **INFERIDO**: Integraciones externas

---

## 7. Query DSL

### 7.1 Operadores

**Booleanos**:
```clojure
(and query*)   ; Intersección
(or query*)    ; Unión
(not query)    ; Negación
```

**Filtros**:
```clojure
(between start end)           ; Rango de fechas
(property key value)          ; Propiedad específica
(task marker*)                ; Filtrar por estado
(priority level*)              ; Filtrar por prioridad
(page "Page Name")            ; Página específica
[[page-ref]]                  ; Referencia directa
(full-text-search "text")     ; Búsqueda full-text
(sample n)                    ; Muestra aleatoria
```

### 7.2 Time Helpers

```clojure
today, yesterday, tomorrow
-7d, +7d    ; Días relativos
-1w, +1w    ; Semanas
-1m, +1m    ; Meses
-1y, +1y    ; Años
-1h, +1h    ; Horas
-1n         ; Minutos
```

---

## 8. Constantes del Sistema

### 8.1 Built-in Pages

```clojure
"Contents"           ; Página de contenido
"logseq/custom.css"  ; CSS personalizado
"logseq/bak/"        ; Backups
```

### 8.2 Reserved Names

**Títulos de página no válidos**:
- No puede contener: `/`, `#`, `?`, `:`, `|`, `<`, `>`, `*`, `"`, `\`
- No puede ser vacío
- No puede ser solo números

### 8.3 View Contexts

```clojure
:all      ; Todas las vistas
:page     ; Contexto de página
:block    ; Contexto de bloque
:class    ; Contexto de clase
```

---

## 9. Validaciones Implícitas

### 9.1 Page Title Validation

```clojure
;; No puede contener: / # ? : | < > * " \
;; No puede ser vacío
;; No puede ser solo números
```

### 9.2 Block UUID

- 🟢 **CONFIRMADO**: No puede cambiar una vez creado
- 🟢 **CONFIRMADO**: Validado en `outliner-core/-save`

### 9.3 Property Names

- 🟢 **CONFIRMADO**: Lowercase after keyword conversion
- 🟢 **CONFIRMADO**: `/` reemplazado por `-`
- 🟢 **CONFIRMADO**: Spaces reemplazados por `-`
- 🟢 **CONFIRMADO**: `_` reemplazado por `-`

---

## 10. Escalas de Confianza

| Símbolo | Significado |
|----------|-------------|
| 🟢 **CONFIRMADO** | Extraído directamente del código o tests |
| 🟡 **INFERIDO** | Deducido de patrones y contexto |
| 🔴 **LACUNA** | No hay suficiente evidencia |

---

*Documento generado automáticamente por Reversa Detective*
