# frontend/search — Búsqueda

## Visão Geral
Sistema de búsqueda full-text con soporte para múltiples motores. Implementa el patrón Agency para coordinar y distribuir queries entre el motor nativo del browser (basado en DataScript/worker) y motores externos via Plugin API. Incluye búsqueda difusa (fuzzy search) con normalización de queries y soporte para iniciales Hanzi.

## Responsabilidades
- Coordinar múltiples motores de búsqueda mediante el patrón Agency
- Ejecutar búsqueda de bloques con fuzzy matching y normalización de acentos
- Buscar archivos por nombre (excluyendo archivos Markdown)
- Buscar templates por título
- Reconstruir índices de búsqueda (páginas y bloques) bajo demanda
- Sincronizar cambios incrementales en los índices (`transact-blocks!`)
- Proveer función de fuzzy search exportable para uso general

## Interface

### Protocolo `Engine`
```clojure
(defprotocol Engine
  (query                  [_this q opts])
  (rebuild-blocks-indice! [_this])
  (rebuild-pages-indice!  [_this])
  (transact-blocks!       [_this data])
  (truncate-blocks!       [_this])
  (remove-db!             [_this]))
```

### Funciones públicas (search.cljs / agency.cljs)

| Función | Parámetros | Retorno | Descripción |
|---------|------------|---------|-------------|
| `get-engine` | `repo` | `Agency` | Crea una Agency que coordina Browser + Plugin engines |
| `block-search` | `repo q option` | `results` | Búsqueda normalizada con fuzzy search |
| `file-search` | `q limit` | `[file]` | Busca en nombres de archivo (excluye `.md`) |
| `template-search` | `q limit` | `[template]` | Busca en templates por título |
| `rebuild-indices!` | `repo` | `promise` | Reconstruye índices de páginas y bloques |
| `fuzzy-search` | `items q opts` | `[result]` | Búsqueda difusa genérica (exportada) |

### Motores de búsqueda

| Motor | Archivo | Descripción |
|-------|---------|-------------|
| **Browser** | `browser.cljs` | Motor nativo que delega a `thread-api/search-blocks` en db-worker |
| **Plugin** | `plugin.cljs` | Motor que invoca plugins via Plugin API |
| **Agency** | `agency.cljs` | Coordinator que distribuye queries a todos los motores registrados |

### Flujo Agency
```clojure
;; Agency.query → Browser.query → Plugin1.query → Plugin2.query → ...
;; Los resultados de cada motor se combinan
(get-registered-engines repo)  ;; => [Browser Plugin1 Plugin2 ...]
```

### Entidades de datos

| Entidad | Campos | Descripción |
|---------|--------|-------------|
| `SearchResult` | `:id` (any), `:content` (string), `:path` (string), `:score` (number) | Resultado individual de búsqueda |
| `SearchEngine` | `:repo` (string) | Motor de búsqueda asociado a un grafo |
| `BlockTransactData` | `:blocks-to-remove-set` (set), `:blocks-to-add` (vector) | Datos de transacción incremental para índices |

## Regras de Negócio
- 🟢 Agency envía queries al Browser engine y luego a todos los Plugin engines registrados (`agency.cljs:22-26`)
- 🟢 `file-search` excluye archivos con extensión `.md` o `.markdown` (`search.cljs:40-41`)
- 🟢 `block-search` normaliza la query usando `fuzzy/search-normalize` y aplica `enable-search-remove-accents?` según configuración (`search.cljs:25`)
- 🟡 `fuzzy-search` exporta la función para uso en otros módulos (ej: búsqueda de páginas en el editor)
- 🟡 El índice de búsqueda se reconstruye programáticamente cuando `input-idle?` (5 segundos sin actividad)

## Fluxo Principal

### Búsqueda de bloques
1. Cliente llama a `block-search` con `repo`, `q` (query string) y `option`
2. La query se normaliza: `fuzzy/search-normalize` + opcionalmente `remove-accents`
3. `get-engine` crea/recupera una Agency para el repo
4. Agency delega `query` al Browser engine (db-worker) y a cada Plugin engine registrado
5. Browser engine ejecuta `thread-api/search-blocks` en el worker thread
6. Los resultados de todos los motores se combinan y retornan

### Reconstrucción de índices
1. Cliente (handler) llama a `rebuild-indices!` con `repo` — típicamente tras `graph/switch` o `input-idle?`
2. Agency invoca `rebuild-pages-indice!` y `rebuild-blocks-indice!` en cada motor
3. Browser engine reconstruye sus índices desde DataScript
4. Plugin engines reconstruyen sus índices si están habilitados

### Transacción incremental
1. Cuando se crean, modifican o eliminan bloques, el handler invoca `transact-blocks!` en la Agency
2. Agency propaga los datos (`blocks-to-remove-set`, `blocks-to-add`) a todos los motores
3. Cada motor actualiza su índice incrementalmente sin rebuild completo

## Fluxos Alternativos
- **Repo sin plugins habilitados:** Agency registra solo el Browser engine; las queries van únicamente a Browser
- **Motor Browser no disponible (db-worker caído):** La query falla y el error se propaga; los Plugin engines aún pueden responder si están activos
- **`file-search` sin resultados:** Retorna vector vacío `[]`
- **Índice corrupto o desincronizado:** Se programa un `rebuild-blocks-indice!` completo vía `schedule-search-index-build!` en el handler de eventos

## Dependências
- `frontend.state` — Estado global para acceder a configuración (`enable-search-remove-accents?`)
- `frontend.db` / `frontend.db.async` — Acceso a datos y worker threads para queries de búsqueda
- `frontend.common.search-fuzzy` — Algoritmo de fuzzy search con soporte Hanzi
- `plugin-api` — API para invocar motores de búsqueda externos (plugins)

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Performance | Búsqueda de bloques delegada a worker thread para no bloquear UI | `browser.cljs` → `thread-api/search-blocks` | 🟢 |
| Escalabilidad | Transacciones incrementales (`transact-blocks!`) evitan rebuild completo en cada cambio | `agency.cljs` método `transact-blocks!` | 🟢 |
| Extensibilidad | Patrón Agency permite agregar nuevos motores sin modificar el código de búsqueda | `agency.cljs` + `get-registered-engines` | 🟢 |
| Latencia | Reconstrucción de índices diferida hasta `input-idle?` (5s sin input) | `handler/events.cljs:73-91` | 🟢 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

```gherkin
Dado un grafo con bloques que contienen "DataScript"
Quando se ejecuta block-search con query "datascript"
Então se retornan los bloques que contienen "DataScript" (case-insensitive)
Y los resultados incluyen score de relevancia

Dado un grafo con archivos "config.edn", "readme.md", "notes.org"
Quando se ejecuta file-search con query "config"
Então se retorna "config.edn"
Y NO se retorna "readme.md" (extensión markdown excluida)

Dado un grafo con 3 plugins de búsqueda registrados
Quando se ejecuta query a través de Agency
Então la query se envía al Browser engine primero
Y luego a cada uno de los 3 Plugin engines
Y los resultados se combinan

Dado que se crea un nuevo bloque en el grafo
Quando se invoca transact-blocks! con ese bloque en blocks-to-add
Então el índice de búsqueda se actualiza incrementalmente
Y el nuevo bloque aparece en búsquedas subsiguientes sin rebuild completo

Dado un índice de búsqueda corrupto o vacío
Quando se invoca rebuild-indices! para el repo
Então los índices de páginas y bloques se reconstruyen completamente
Y búsquedas posteriores retornan resultados correctos

Dado un repo sin plugins de búsqueda habilitados
Quando se ejecuta block-search
Então solo el Browser engine procesa la query
Y los resultados se retornan normalmente
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| `block-search` con fuzzy matching | Must | Funcionalidad core — búsqueda de contenido es esencial en un PKM |
| `get-engine` / Agency | Must | Punto de entrada único para toda búsqueda en el sistema |
| `file-search` | Must | Usado en diálogos de selección de archivos y navegación |
| `rebuild-indices!` | Must | Necesario tras carga de grafo y switches de repo |
| `transact-blocks!` | Should | Optimización importante, pero el sistema funciona con rebuild completo |
| `fuzzy-search` (exportada) | Should | Usada por otros módulos (editor, page search); no crítica por sí sola |
| `template-search` | Could | Funcionalidad auxiliar para diálogo de templates |
| Plugin engines | Could | Extensibilidad vía plugins; el sistema funciona solo con Browser engine |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `src/main/frontend/search.cljs` | `get-engine`, `block-search`, `file-search`, `template-search`, `rebuild-indices!`, `fuzzy-search` | 🟢 |
| `src/main/frontend/search/protocol.cljs` | Protocolo `Engine` (6 métodos) | 🟢 |
| `src/main/frontend/search/agency.cljs` | `query`, `rebuild-blocks-indice!`, `rebuild-pages-indice!`, `transact-blocks!`, `truncate-blocks!`, `remove-db!` | 🟢 |
| `src/main/frontend/search/browser.cljs` | `query` (→ thread-api/search-blocks), `transact-blocks!` | 🟢 |
| `src/main/frontend/search/plugin.cljs` | Motor vía Plugin API | 🟡 |

## Cenários de Borda

### Query con solo caracteres especiales o símbolos
- **Contexto**: Usuario busca `*`, `#`, `@` o strings puramente simbólicos
- **Comportamiento**: `fuzzy/search-normalize` procesa la query. Caracteres no alfanuméricos pueden ser eliminados durante la normalización, resultando en query vacía → sin resultados. El comportamiento exacto depende de la implementación de `search-fuzzy`

### Búsqueda con acentos y configuración `enable-search-remove-accents?`
- **Contexto**: Usuario busca "acción" con la configuración de remover acentos activada
- **Comportamiento**: Con `remove-accents? = true`, la query se normaliza a "accion" y matchea tanto "acción" como "accion". Con `false`, solo matchea "acción" exactamente. Esta configuración es global por grafo

### Motor Browser vs Plugin con resultados contradictorios
- **Contexto**: Browser engine retorna 5 resultados, Plugin engine retorna 3 resultados diferentes para la misma query
- **Comportamiento**: Agency combina todos los resultados. No hay deduplicación ni resolución de conflictos entre motores — cada motor es responsable de sus propios resultados. El consumidor (handler/search.cljs) puede aplicar post-procesamiento

### Índice masivo con cientos de miles de bloques
- **Contexto**: Grafo con 100,000+ bloques tras años de uso
- **Comportamiento**: `rebuild-indices!` completo puede ser costoso. El sistema mitiga esto con `transact-blocks!` incremental. La reconstrucción completa ocurre solo en eventos explícitos (graph switch, restore). El worker thread evita bloquear la UI durante el rebuild
