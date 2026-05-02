# graph-parser

## Visão Geral
Librería externa (`deps/graph-parser`) responsable de convertir archivos Markdown y Org-mode en estructuras de datos compatibles con DataScript. Es el puente entre el sistema de archivos y la base de datos, extrayendo páginas, bloques, propiedades, referencias y metadata temporal desde el contenido raw de los archivos.

## Responsabilidades
- Parsear archivos Markdown/Org a Abstract Syntax Tree (AST) vía Mldoc
- Extraer páginas con detección de título por prioridad (property → filename → first heading)
- Extraer bloques, propiedades, referencias y timestamps desde el AST
- Detectar journals mediante patrones de fecha configurables
- Resolver referencias a páginas y bloques existentes o crear placeholders
- Corregir IDs duplicados de bloques en re-parsing
- Asignar orden lexicográfico y relaciones parent/child entre bloques

## Interface

### Entrada principal: `extract`
```clojure
(extract file-path content {:keys [user-config verbose]})
;; file-path: String  — ruta del archivo
;; content:   String  — contenido raw del archivo
;; options:   Map     — user-config (config de usuario), verbose (bool)
;; Retorna:   {:pages [Page] :blocks [Block] :ast AST}
```

### Tipos de datos de entrada/salida

**Page (Mapa de página extraída):**
```clojure
{:block/name        String?    ;; nombre canónico
 :block/uuid        UUID       ;; UUID generado
 :block/title       String?    ;; título detectado
 :block/tags        [Ref]?     ;; tags/clases
 :block/format      Keyword    ;; :markdown | :org
 :block/journal-day Int?       ;; YYYYMMDD si es journal
 :block/properties  Map?       ;; propiedades built-in (title::, alias::, etc.)
 :block/file        {:file/path String}}
```

**Block (Mapa de bloque extraído):**
```clojure
{:block/uuid        UUID
 :block/title       String?
 :block/format      Keyword         ;; :markdown | :org
 :block/level       Int             ;; nivel de indentación (1-indexed)
 :block/order       String          ;; orden lexicográfico
 :block/parent      {:db/id Int}?   ;; referencia al bloque padre
 :block/page        {:db/id Int}?   ;; referencia a la página contenedora
 :block/refs        [Ref]?          ;; referencias a otras páginas/bloques
 :block/tags        [Ref]?          ;; tags/clases
 :block/properties  Map?            ;; mapa de propiedades
 :block/pre-block?  Boolean?        ;; bloque decorativo pre-content
 :block/timestamps  [Timestamp]?    ;; timestamps extraídos (SCHEDULED, DEADLINE)}
```

## Regras de Negócio
- Título de página: se determina por prioridad — `title::` property > file name parsing > first heading content 🟢
- Journal detection: un archivo es journal si su nombre matchea con un patrón de fecha configurable (`journal-title->int`) 🟢
- Bloques duplicados: si se detecta un UUID ya existente en `*extracted-block-ids`, se genera un nuevo UUID automáticamente 🟢
- Propiedades inválidas: propiedades desconocidas o con keys no válidas se acumulan en `:invalid-properties` y no se descartan 🟢
- Referencias a páginas: si una página referenciada no existe en DB, se crea un placeholder con UUID generado 🟡
- Timestamps: se acumulan durante el parsing y se asocian al último heading encontrado 🟢
- Formato del archivo: se detecta por extensión o por la primera línea (`get-format`) → `:markdown` o `:org` 🟢
- Bloques tipo property-drawer: se mergean con el bloque anterior si son consecutivos 🟢
- Bloques pre-block: bloques decorativos (barras, separadores) se adjuntan al bloque previo en lugar de crear uno nuevo 🟡

## Fluxo Principal

### Pipeline de extracción
1. Recibir `file-path` y `content` del archivo
2. Detectar formato (`:markdown` | `:org`) vía `common-util/get-format`
3. Convertir contenido a AST vía `mldoc/->edn(content, format-config)`
4. Determinar nombre de página:
   a. Si el primer bloque tiene propiedad `title::`, usar ese valor
   b. Si no, parsear el nombre del archivo según `filename-format` (`:triple-lowbar` o `:legacy`)
   c. Si no hay título ni heading, usar el nombre del archivo decodificado
5. Extraer propiedades del primer bloque (property drawer)
6. Extraer bloques del AST vía `extract-blocks`:
   a. Recorrer nodos del AST secuencialmente
   b. Para cada nodo: clasificar como Heading, Property, Timestamp, Paragraph, u Other
   c. Acumular timestamps y propiedades
   d. Construir `construct-block` con metadata acumulada
   e. Asignar `block/level` según profundidad de heading
7. Construir `build-page-map` con propiedades y metadata
8. Resolver referencias: `with-ref-pages` para páginas referenciadas no existentes
9. Asignar `with-parent-and-order`: establecer `block/parent` y `block/order` jerárquico
10. Corregir `fix-block-id-if-duplicated!` si hay UUIDs ya existentes en DB
11. Retornar `{:pages [...] :blocks [...] :ast ast}`

## Fluxos Alternativos
- **[Archivo sin contenido]:** Si `content` es nil o vacío, `extract` retorna `{:pages [] :blocks [] :ast nil}` 🟡
- **[Formato no soportado]:** Si el formato detectado no es `:markdown` ni `:org`, se lanza excepción `ex-info` con mensaje descriptivo 🔴
- **[Journal con formato personalizado]:** Si el archivo es journal con un formato de fecha distinto al default, se usa `convert-page-if-journal` para normalizar el nombre y calcular `journal-day` 🟡
- **[Propiedades con valores de referencia]:** Propiedades tipo ref (`[[page-ref]]`) se parsean a `:block/refs` en lugar de tratarse como strings 🟢
- **[Bloques con IDs duplicados en re-parse]:** Si se re-parsea un archivo y un bloque tiene UUID ya registrado, se genera uno nuevo, se actualiza `*extracted-block-ids` y se loggea warning 🟢
- **[Namespace pages]:** Páginas con `/` en el nombre se tratan como namespaces; `build-pages-aux` crea la jerarquía namespace→page 🟢

## Dependências
- `mldoc` (librería externa) — parsing de Markdown/Org a AST
- `datascript.core` — consultas `d/entity`, `d/datoms` para verificar existencia de páginas/bloques
- `logseq.db` — funciones `ldb/get-page`, `ldb/page-exists?` para resolución de referencias
- `logseq.common.util` — utilidades: `get-format`, `safe-decode-uri-component`, `page-name-sanity`

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Performance | Parsing de archivos grandes se hace en un solo paso (single-pass sobre AST) | `deps/graph-parser/src/logseq/graph_parser/extract.cljc` | 🟢 |
| Robustez | IDs duplicados se detectan y corrigen automáticamente sin fallar | `deps/graph-parser/src/logseq/graph_parser/block.cljs:736-745` | 🟢 |
| Extensibilidad | Detección de formato extensible vía `get-format` con soporte para markdown y org | `deps/graph-parser/src/logseq/graph_parser/extract.cljc` | 🟢 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

### Cenário: Extracción de página Markdown estándar
```gherkin
Dado un archivo "my-page.md" con contenido:
  ```
  title:: My Custom Title
  tags:: tag1, tag2
  - First bullet
  - Second bullet
  ```
Cuando se llama a `extract("my-page.md", content, {:user-config default-config})`
Então el resultado contiene:
  - Una página con `:block/name` = "my custom title"
  - `:block/tags` con referencias a "tag1" y "tag2"
  - Dos bloques extraídos con `:block/level` = 1
  - Ningún bloque marcado como pre-block
```

### Cenário: Detección de journal por fecha
```gherkin
Dado un archivo "2026-05-02.md" con contenido:
  ```
  - Task for today
  ```
  Y el formato de journal configurado como "yyyy-MM-dd"
Cuando se llama a `extract("2026-05-02.md", content, {:user-config {:journal/page-title-format "yyyy-MM-dd"}})`
Então la página resultante tiene `:block/journal-day` = 20260502
  Y `:block/tags` incluye referencia a `:logseq.class/Journal`
```

### Cenário: Corrección de bloque con UUID duplicado
```gherkin
Dado un archivo "page.md" con contenido:
  ```
  - Block with explicit id:: 550e8400-e29b-41d4-a716-446655440000
  ```
  Y la DB ya contiene un bloque con UUID "550e8400-..."
Cuando se llama a `extract` con `*extracted-block-ids` conteniendo ese UUID
Então se genera un nuevo UUID para el bloque conflictivo
  Y se emite un warning de log
  Y el bloque original en DB no se modifica
```

### Cenário: Archivo vacío
```gherkin
Dado un archivo "empty.md" con contenido "" (string vacío)
Cuando se llama a `extract("empty.md", "", {:user-config default-config})`
Então se retorna `{:pages [] :blocks [] :ast nil}`
  Y no se lanza ninguna excepción
```

### Cenário: Propiedades inválidas no bloquean la extracción
```gherkin
Dado un archivo "page.md" con contenido:
  ```
  title:: Valid Title
  invalid/key:: some value
  - Content block
  ```
Cuando se llama a `extract`
Então la página se crea con título "Valid Title"
  Y las propiedades válidas se incluyen en `:block/properties`
  Y la propiedad `invalid/key` se acumula en `:invalid-properties`
  Y los bloques de contenido se extraen normalmente
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| Parseo Markdown/Org a bloques | Must | Camino crítico — sin esto no hay datos en la app |
| Detección de título de página | Must | Requerido para toda página; usado en indexación y búsqueda |
| Detección de journal | Must | Journals son páginas especiales del núcleo de la app |
| Extracción de propiedades | Must | Propiedades son el sistema de metadatos principal |
| Resolución de referencias | Should | Importante para navegación pero puede diferirse con placeholders |
| Corrección de UUIDs duplicados | Should | Caso de borde que ocurre solo en re-parsing |
| Soporte para namespaces | Should | Funcionalidad importante pero no bloqueante |
| Timestamps en bloques | Could | Solo relevante para bloques con SCHEDULED/DEADLINE |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Cenários de Borda

### 1. Título de página con caracteres especiales
**Situação:** Archivo con nombre que contiene `%20`, `+`, caracteres Unicode, o múltiples puntos (ej: `my.page.v2.md`).
**Comportamento esperado:**
- `safe-decode-uri-component` decodifica caracteres URL-encoded
- `page-name-sanity/lower-case` normaliza a lowercase
- Puntos en formato legacy se reemplazan por `/`; en triple-lowbar se preservan
- Caracteres Unicode se preservan sin modificaciones (ej: 日本語, 한국어)

### 2. Archivo con múltiples property drawers
**Situação:** Archivo que contiene más de un bloque de propiedades (`:PROPERTIES: ... :END:`).
**Comportamento esperado:**
- Solo el primer property drawer se usa para propiedades de página
- Property drawers posteriores se tratan como bloques de contenido si están bajo un heading
- Si hay múltiples property drawers consecutivos sin heading intermedio, se mergean en uno solo

### 3. Bloque con referencias circulares de página
**Situação:** Archivo donde la página "A" referencia a "B" y "B" referencia a "A" en el mismo batch de parsing.
**Comportamento esperado:**
- La primera ocurrencia crea la página normalmente
- La referencia a la página aún no existente crea un placeholder con UUID
- Cuando la segunda página se procesa, el placeholder se actualiza con los datos reales
- No se produce loop infinito porque la resolución es por nombre, no por ID

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `deps/graph-parser/src/logseq/graph_parser/extract.cljc` | `extract` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/extract.cljc` | `extract-pages-and-blocks` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/extract.cljc` | `title-parsing` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/extract.cljc` | `build-page-map` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/extract.cljc` | `with-ref-pages` | 🟡 |
| `deps/graph-parser/src/logseq/graph_parser/block.cljs` | `extract-blocks` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/block.cljs` | `get-page-reference` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/block.cljs` | `construct-block` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/block.cljs` | `fix-block-id-if-duplicated!` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/block.cljs` | `with-parent-and-order` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/block.cljs` | `heading-block?` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/block.cljs` | `extract-properties` | 🟡 |
| `deps/graph-parser/src/logseq/graph_parser/mldoc.cljc` | `->edn` | 🟡 |
| `deps/graph-parser/src/logseq/graph_parser/property.cljs` | `extract-properties` | 🟢 |
| `deps/graph-parser/src/logseq/graph_parser/text.cljs` | Text utilities | 🟡 |
