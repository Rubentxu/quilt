# frontend/format — Parsers (Markdown / Org-mode)

## Visão Geral
Módulo de parsing y serialización de formatos Markdown y Org-mode. Actúa como wrapper del parser externo `mldoc`, proporcionando conversión bidireccional entre contenido raw y AST (Abstract Syntax Tree), renderizado a HTML, y exportación a Markdown/OPML. Incluye caché LRU para optimizar el parsing de bloques.

## Responsabilidades
- Convertir contenido Markdown/Org a AST (EDN) mediante `mldoc`
- Renderizar AST a HTML para visualización en el editor
- Exportar contenido a Markdown y OPML
- Extraer texto plano de elementos AST (`plain->text`)
- Detectar y extraer queries custom embebidas en el AST
- Parsear bloques individuales separando título y cuerpo
- Mantener caché LRU de resultados de parsing para evitar re-procesamiento

## Interface

### Protocolo `Format`
```clojure
(defprotocol Format
  (toEdn          [_this content config])
  (toHtml         [_this content config references])
  (exportMarkdown [_this content config references])
  (exportOPML     [_this content config title references]))
```

### Funciones públicas (format.cljs / mldoc.cljs / block.cljs)

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `to-html` | `content config` | `html-string` | Convierte contenido a HTML; retorna `""` si content es blank |
| `to-edn` | `content config` | `ast` | Convierte contenido a AST/EDN vía mldoc |
| `parse-export-markdown` | `content config references` | `string` | Exporta AST a Markdown |
| `parse-export-opml` | `content config title references` | `string` | Exporta AST a OPML |
| `plain->text` | `plains` | `string` | Extrae texto plano de nodos Plain del AST |
| `extract-first-query-from-ast` | `ast` | `query-string` | Encuentra la primera query custom en el AST usando postwalk |
| `extract-blocks` | `blocks content format {:keys [page-name]}` | `[block]` | Extrae bloques del AST con manejo de errores |
| `parse-block` | `{:block/keys [uuid title format]}` | `block` | Parsea título de bloque, extrae AST y remueve `:block/pre-block?` |
| `parse-title-and-body` | `block-uuid format content` | `{:block.temp/ast-body :block.temp/ast-title}` | Separa título y cuerpo usando LRU cache |
| `trim-break-lines!` | `ast` | `ast` | Limpia line breaks especiales de nodos Paragraph |

### Entidades de datos

| Entidad | Campos | Descripción |
|---------|--------|-------------|
| `MldocAST` | `:type` (string), `:metadata` (map), `:content` (any) | Nodo del AST producido por mldoc: `["Heading" {:size 1} [["Plain" "Title"]]]` |
| `BlockParseResult` | `:block.temp/ast-body` (vector), `:block.temp/ast-title` (string) | Resultado de `parse-title-and-body` |

### Formatos soportados
- `:markdown` / `:md` — Markdown estándar
- `:org` — Org-mode

### Estructura típica del AST (Mldoc)
```clojure
["Heading" {:size 1 :anchor "anchor"} [["Plain" "Title"]]]    ;; Encabezado
["Paragraph" [["Plain" "text"]]]                               ;; Párrafo
["Drawer" "properties" [...]]                                  ;; Cajón de propiedades
["Code" {:language "clojure"} "code content"]                  ;; Bloque de código
["Block_reference" "uuid"]                                     ;; Referencia a bloque
```

## Regras de Negócio
- 🟢 `to-html` retorna string vacío si el contenido de entrada es blank (`format.cljs:13-14`)
- 🟢 `parse-block` remueve el campo `:block/pre-block?` del resultado (`block.cljs:50`)
- 🟢 `parse-title-and-body` utiliza caché LRU con threshold de 5000 entradas (`block.cljs:66`)
- 🟢 La detección de heading para separar título/cuerpo busca el primer nodo `Heading` en el AST
- 🟡 El formato se hereda de la página padre al bloque si no se especifica explícitamente
- 🟡 `extract-first-query-from-ast` usa `clojure.walk/postwalk` para recorrer el AST completo

## Fluxo Principal

### Parsing de contenido a HTML
1. Cliente llama a `to-html` con `content` (string markdown/org) y `config`
2. Si el contenido es blank/empty, retorna `""` inmediatamente
3. `mldoc` convierte el contenido a AST
4. El AST se renderiza a HTML usando el motor interno de mldoc
5. Retorna el string HTML resultante

### Parsing de bloque individual
1. Cliente llama a `parse-block` con `{:block/uuid :block/title :block/format}`
2. `parse-title-and-body` separa el contenido en título (primer heading) y cuerpo (resto)
3. El AST del cuerpo se extrae vía mldoc con el formato indicado
4. Se remueve `:block/pre-block?` del resultado
5. Retorna el bloque con AST poblado

### Exportación a Markdown
1. Cliente llama a `parse-export-markdown` con `content`, `config`, `references`
2. El contenido se parsea a AST vía mldoc
3. El AST se serializa de vuelta a Markdown respetando referencias
4. Retorna el string Markdown

## Fluxos Alternativos
- **Contenido blank/vacío en `to-html`:** Retorna `""` sin llamar a mldoc
- **Error de parsing en mldoc:** `extract-blocks` captura el error y retorna los bloques en estado fallback
- **Query no encontrada en AST:** `extract-first-query-from-ast` retorna `nil`
- **Entrada LRU cache en `parse-title-and-body`:** Si el bloque ya fue parseado (mismo UUID + content hash), retorna resultado cacheado sin re-procesar
- **Bloque sin heading:** `parse-title-and-body` trata todo el contenido como cuerpo, título vacío

## Dependências
- `mldoc` (librería externa) — Parser principal de Markdown y Org-mode. Convierte entre texto y AST.
- `logseq.graph-parser.mldoc` — Wrapper compartido de mldoc usado también por graph-parser
- `clojure.walk` — Funciones `postwalk`/`prewalk` para recorrer el AST en búsqueda de queries y headings

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Performance | Caché LRU de 5000 entradas para `parse-title-and-body` evita re-parsing | `frontend/format/block.cljs:66` | 🟢 |
| Robustez | `extract-blocks` captura excepciones de mldoc y retorna fallback | `frontend/format/block.cljs` (try/catch) | 🟢 |
| Rendimiento | Short-circuit en `to-html` para contenido blank evita llamada innecesaria a mldoc | `frontend/format.cljs:13-14` | 🟢 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

```gherkin
Dado un bloque con contenido Markdown "# Título\nCuerpo del bloque" y formato :markdown
Cuando se llama a parse-block con los datos del bloque
Então el resultado incluye :block.temp/ast-title = "Título"
Y :block.temp/ast-body contiene el AST del cuerpo "Cuerpo del bloque"

Dado un contenido Markdown "# Hello World"
Cuando se llama a to-html con ese contenido
Então se retorna un string HTML que contiene un elemento h1 con "Hello World"

Dado un contenido vacío o blank
Cuando se llama a to-html
Então se retorna "" sin invocar a mldoc

Dado un AST que contiene una query custom (bloque con #+BEGIN_QUERY)
Cuando se llama a extract-first-query-from-ast
Então se retorna el string de la query encontrada

Dado un AST sin ninguna query custom
Cuando se llama a extract-first-query-from-ast
Então se retorna nil

Dado un bloque parseado previamente (mismo UUID + contenido)
Cuando se llama a parse-title-and-body por segunda vez
Então se retorna el resultado de la caché LRU sin re-procesar con mldoc
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| `to-edn` / `to-html` | Must | Conversión fundamental para visualización de todo contenido en la app |
| `parse-block` / `parse-title-and-body` | Must | Llamado en cada edición de bloque para actualizar AST |
| `extract-blocks` | Must | Usado por graph-parser para cargar archivos en la DB |
| `plain->text` | Should | Necesario para búsqueda e indexación; existe alternativa vía regex simple |
| `parse-export-markdown` | Should | Usado en exportación de páginas; no en flujo normal de edición |
| `parse-export-opml` | Could | Formato de exportación especializado, raramente usado |
| `extract-first-query-from-ast` | Could | Solo relevante para bloques con queries custom embebidas |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `src/main/frontend/format.cljs` | `to-html`, `to-edn` | 🟢 |
| `src/main/frontend/format/protocol.cljs` | Protocolo `Format` (4 métodos) | 🟢 |
| `src/main/frontend/format/mldoc.cljs` | `parse-export-markdown`, `parse-export-opml`, `plain->text`, `extract-first-query-from-ast` | 🟢 |
| `src/main/frontend/format/block.cljs` | `extract-blocks`, `parse-block`, `parse-title-and-body`, `trim-break-lines!` | 🟢 |

## Cenários de Borda

### Bloque con markup malformado
- **Contexto**: Contenido con sintaxis Markdown/Org inválida (ej: negritas no cerradas `**texto`)
- **Comportamiento**: `mldoc` aplica recuperación de errores interna (error recovery). El AST resultante puede contener nodos `"Paragraph"` con el texto raw. `extract-blocks` captura excepciones y no interrumpe el procesamiento del archivo completo

### Bloque extremadamente largo (miles de líneas)
- **Contexto**: Un solo bloque con contenido muy extenso (ej: logs pegados)
- **Comportamiento**: La caché LRU de 5000 entradas puede expulsar entradas previas si hay muchos bloques grandes. El parsing con `mldoc` es síncrono y bloquea el thread principal durante el procesamiento. No hay límite de tamaño explícito en el código

### Contenido mixto Markdown/Org en un mismo grafo
- **Contexto**: Un grafo contiene archivos `.md` y `.org` simultáneamente
- **Comportamiento**: El formato se detecta por archivo (extensión o propiedad `:block/format`). `to-edn` y `to-html` reciben el formato como parámetro explícito. No hay conversión automática entre formatos; cada bloque mantiene su formato individual

### Referencias circulares en exportación
- **Contexto**: Dos páginas se referencian mutuamente (`[[A]]` en B, `[[B]]` en A) durante exportación Markdown
- **Comportamiento**: `parse-export-markdown` recibe `references` como parámetro. La resolución de referencias la maneja el llamador (graph-parser o handler). El módulo format solo serializa el AST; no resuelve ni detecta ciclos
