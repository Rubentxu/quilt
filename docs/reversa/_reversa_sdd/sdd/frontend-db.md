# frontend/db

## Visão Geral
Módulo de acceso a datos de Logseq que gestiona la conexión con DataScript y proporciona una interfaz completa para consultas, transacciones y persistencia. Actúa como capa de abstracción sobre `logseq.db` ofreciendo funciones síncronas, asíncronas (worker thread) y reactivas (Rum-aware) para que los componentes de UI y handlers puedan leer y escribir datos sin conocer los detalles de DataScript. Incluye un motor DSL de queries que traduce expresiones tipo `(and (task TODO) (priority A))` a Datalog.

## Responsabilidades
- Gestionar el ciclo de vida de las conexiones DataScript (`start!`, `get-db`, restauración)
- Ejecutar transacciones batch con soporte para operaciones del outliner (`transact!`, `transact`, `apply-outliner-ops`)
- Proveer acceso directo a entidades por ID, UUID o nombre (`entity`, `pull`, `pull-many`, `get-block-by-uuid`, `get-page`)
- Navegar la estructura jerárquica de bloques (`get-next`, `get-prev`, `get-block-parent`, `has-children?`, `get-block-immediate-children`)
- Resolver queries DSL con sintaxis simplificada (`query`, `parse`, `build-query`, `pre-transform`)
- Ejecutar queries personalizadas con reglas Datalog (`custom-query`)
- Proveer queries reactivas que auto-refrescan componentes Rum (`q`, `react-query`, `refresh!`)
- Gestionar el cache de queries reactivas con invalidación selectiva (`refresh-affected-queries!`)
- Ejecutar queries en worker thread para no bloquear UI (`<get-block`, `<get-blocks`, y demás funciones async)
- Gestionar persistencia multi-backend (IndexedDB, SQLite, archivos locales) (`get-all-graphs`, `delete-graph!`, `restore-graph!`)
- Transformar resultados de queries vía `sci/eval-string` para agrupación y filtrado avanzado

## Interface

### Capa de Conexión (`conn.cljs`, `conn_state.cljs`)

```clojure
;; Obtener DB actual (inmutable, snapshot)
(get-db repo deref?)
;; repo:   String   — URL del repositorio
;; deref?: Boolean? — si debe dereferenciar el átomo (default true)
;; Retorna: DataScript DB

;; Transacción síncrona directa
(transact! repo tx-data tx-meta)
;; repo:    String    — URL del repositorio
;; tx-data: [TxDatum] — datos de transacción
;; tx-meta: Map?      — metadatos de transacción
;; Retorna: nil (efectos secundarios)

;; Inicializar conexión DataScript
(start! repo opts)
;; repo: String — URL del repositorio
;; opts: Map    — opciones de inicialización
;; Retorna: DataScript Connection
```

### Capa de Transacciones (`transact.cljs`)

```clojure
;; Transacción asíncrona (worker thread)
(transact worker-transact repo tx-data tx-meta)
;; worker-transact: Fn     — función de transacción en worker
;; repo:            String — URL del repositorio
;; tx-data:         [TxDatum]
;; tx-meta:         Map?
;; Retorna: Promise<Map> — {:tx-report ...}

;; Aplicar operaciones del outliner
(apply-outliner-ops conn ops opts)
;; conn: DataScript Conn — conexión activa
;; ops:  [OutlinerOp]    — operaciones del outliner
;; opts: Map             — opciones adicionales
;; Retorna: Promise<Map>
```

### Capa de Modelo (`model.cljs`)

```clojure
;; Acceso por UUID
(get-block-by-uuid id)
;; id: UUID|String — UUID del bloque
;; Retorna: Entity | nil

(query-block-by-uuid id)
;; id: UUID|String
;; Retorna: Entity (bloque o página) | nil

;; Acceso a páginas
(get-page page-id-name-or-uuid)
;; page-id-name-or-uuid: Int|String|UUID
;; Retorna: Entity | nil

(get-journal-page page-name)
;; page-name: String — nombre de página journal (ej: "May 2nd, 2026")
;; Retorna: Entity | nil

(get-journal-page-by-day journal-day)
;; journal-day: Int — YYYYMMDD
;; Retorna: Entity | nil

(get-today-journal-page)
;; Retorna: Entity | nil

;; Verificación de existencia
(page-exists? page-name tags)
;; page-name: String
;; tags:      [String]?
;; Retorna: Boolean

;; Navegación de bloques
(get-next db db-id opts)
;; db:    DataScript DB
;; db-id: Int — ID del bloque actual
;; opts:  Map — {:keys [collapse?]}
;; Retorna: Entity (siguiente bloque visible) | nil

(get-prev db db-id)
;; db:    DataScript DB
;; db-id: Int
;; Retorna: Entity (bloque anterior) | nil

;; Jerarquía de bloques
(has-children? block-id)
;; block-id: Int|UUID
;; Retorna: Boolean

(get-block-parent repo block-id)
;; Retorna: Entity | nil

(get-block-immediate-children repo block-uuid)
;; Retorna: [Entity] — hijos directos

(get-block-and-children repo block-uuid opts)
;; Retorna: [Entity] — bloque + descendientes recursivos

;; Bloques de página
(get-page-blocks-no-cache repo page-id opts)
;; Retorna: [Entity] — bloques de la página en orden

(get-page-blocks-count repo page-id)
;; Retorna: Int

;; Clases y propiedades
(get-all-classes repo opts)
;; Retorna: [Entity] — todas las entidades de tipo clase

(get-all-properties graph opts)
;; Retorna: [Entity] — todas las propiedades definidas

(get-structured-children repo eid)
;; Retorna: [Int] — IDs de hijos estructurados (para clases)

(get-class-objects repo class-id)
;; Retorna: [Entity] — instancias de una clase

;; Ordenamiento recursivo
(sort-by-order-recursive form)
;; form: [Entity] — lista de bloques
;; Retorna: [Entity] — bloques ordenados recursivamente por :block/order

(get-latest-journals repo-url n)
;; Retorna: [Entity] — N journals más recientes
```

### Capa DSL de Queries (`query_dsl.cljs`)

```clojure
;; Query DSL completa
(query repo query-string opts)
;; repo:         String  — URL del repositorio
;; query-string: String  — ej: "(and (task TODO) (priority A))"
;; opts:         Map     — {:keys [limit ...]}
;; Retorna: [Entity] — resultados de la query

;; Parseo de query string a Datalog
(parse s db opts)
;; s:    String — query DSL a parsear
;; db:   DataScript DB
;; opts: Map
;; Retorna: {:query [...] :rules [...] :sort-by [...] :blocks? bool :sample int?}

;; Construcción recursiva de query
(build-query e env level)
;; e:     Form   — expresión DSL (lista anidada)
;; env:   Map    — entorno de variables
;; level: Int    — nivel de anidación
;; Retorna: {:query [...] :rules [...]}

;; Pre-transformación del string de query
(pre-transform s)
;; s: String — query string raw
;; Retorna: String — query con time helpers resueltos

;; Query custom
(custom-query repo query-m query-opts)
;; query-m:   Map    — query Datalog estructurada
;; query-opts: Map   — opciones adicionales
;; Retorna: query results
```

### Capa Reactiva (`react.cljs`, `query_react.cljs`)

```clojure
;; Query reactiva (Rum-aware)
(q repo k query-opts query inputs*)
;; repo:       String — URL del repositorio
;; k:          Keyword|String — clave única de cache
;; query-opts: Map — {:keys [query-fn inputs-fn transform-fn]}
;; query:      Vector — query Datalog
;; inputs*:    [Any] — parámetros de la query
;; Retorna: Atom<result> — átomo que se actualiza reactivamente

;; React query con DSL
(react-query repo query-m query-opts)
;; query-m:   Map — {:query string :inputs [Any] :transform-fn Fn}
;; query-opts: Map
;; Retorna: [key result-atom]

;; Invalidación de cache
(refresh! repo-url affected-keys)
;; repo-url:      String
;; affected-keys: [Keyword|String]
;; Retorna: nil (efectos secundarios)

(refresh-affected-queries! repo-url affected-keys opts)
;; Retorna: nil

;; Ejecutar queries custom cuando idle
(run-custom-queries-when-idle!)
;; Retorna: chan (core.async channel)
```

### Capa Async (`async.cljs`, `async/util.cljs`)

```clojure
;; Versiones async de funciones síncronas
(<get-block graph id-uuid-or-name opts)
;; graph: String — repositorio
;; id-uuid-or-name: UUID|String|Int
;; opts: Map
;; Retorna: Promise<Entity>

(<get-blocks graph ids opts)
;; ids: [UUID|String|Int]
;; Retorna: Promise<[Entity]>

(<get-block-parents graph id depth)
;; Retorna: Promise<Entity>

(<get-block-refs graph eid)
;; Retorna: Promise<[Ref]>

(<get-block-refs-count graph eid)
;; Retorna: Promise<Int>

(<get-date-scheduled-or-deadlines journal-title)
;; Retorna: Promise<[Entity]>

(<get-files graph)
;; Retorna: Promise<[File]>

(<get-tag-objects graph class-id)
;; Retorna: Promise<[Entity]>

(<task-spent-time graph block-id)
;; Retorna: Promise<[StatusHistory Time]>

(<get-asset-with-checksum graph checksum)
;; Retorna: Promise<Entity>
```

### Capa de Persistencia (`persist.cljs`, `restore.cljs`)

```clojure
;; Obtener todos los grafos
(get-all-graphs)
;; Retorna: Promise<[{:graph-name String :url String ...}]>

;; Eliminar un grafo
(delete-graph! graph)
;; graph: String — nombre/URL del grafo
;; Retorna: Promise

;; Restaurar un grafo desde backup o remoto
(restore-graph! repo opts)
;; repo: String — URL del repositorio
;; opts: Map
;; Retorna: Promise
```

### Tipos de datos principales

**Block (entidad DataScript):**
```clojure
{:db/id              Int         ;; ID interno
 :db/ident           Keyword?    ;; ident único global (:logseq.class/Journal)
 :block/uuid         UUID        ;; UUID inmutable (unique identity)
 :block/parent       {:db/id Int}? ;; bloque padre (indexado)
 :block/page         {:db/id Int}? ;; página contenedora (indexado)
 :block/order        Int?        ;; orden entre siblings (indexado)
 :block/collapsed?   Boolean?    ;; estado de colapso
 :block/name         String?     ;; nombre canónico (indexado, solo páginas)
 :block/title        String?     ;; título visible (indexado)
 :block/content      String?     ;; contenido raw
 :block/format       Keyword?    ;; :markdown | :org
 :block/refs         [{:db/id Int}]? ;; referencias entrantes (cardinality many)
 :block/tags         [{:db/id Int}]? ;; tags/clases (cardinality many)
 :block/link         {:db/id Int}? ;; enlace asociado (indexado)
 :block/alias        [{:db/id Int}]? ;; alias (cardinality many, indexado)
 :block/journal-day  Int?        ;; YYYYMMDD (indexado)
 :block/level        Int?        ;; nivel de indentación
 :block/tx-id        Int?        ;; ID de transacción
 :block/created-at   Long?       ;; timestamp creación (indexado)
 :block/updated-at   Long?       ;; timestamp actualización (indexado)
 :block/closed-value-property [{:db/id Int}]? ;; propiedades closed-value
 :block/properties   Map?}       ;; mapa de propiedades key-value
```

**Page (hereda de Block):**
```clojure
{:db/id            Int
 :db/ident         Keyword?
 :block/uuid       UUID        ;; unique identity
 :block/name       String      ;; nombre canónico (indexado)
 :block/title      String?     ;; título visible (indexado)
 :block/page       {:db/id Int}? ;; namespace padre
 :block/parent     {:db/id Int}?
 :block/alias      [{:db/id Int}]? ;; alias (cardinality many, indexado)
 :block/tags       [{:db/id Int}]? ;; tags (cardinality many)
 :block/journal-day Int?       ;; YYYYMMDD (indexado)
 :block/namespace  {:db/id Int}?} ;; namespace contenedor
```

**File:**
```clojure
{:db/id               Int
 :file/path           String    ;; ruta del archivo (unique identity)
 :file/content        String?   ;; contenido raw
 :file/created-at     Long?
 :file/last-modified-at Long?
 :file/size           Int?}
```

**QueryCacheEntry:**
```clojure
{:query       Vector    ;; query Datalog
 :inputs      [Any]     ;; parámetros de la query
 :result      Atom      ;; átomo con resultado actual
 :transform-fn Fn?      ;; función de transformación
 :query-fn    Fn?       ;; función para re-ejecutar query
 :inputs-fn   Fn?}      ;; función para obtener inputs actualizados
```

## Regras de Negócio
- UUID de bloque no puede cambiar una vez creado — validado durante `save-block` del outliner 🟢
- Bloques built-in (páginas como "Contents", "logseq/custom.css") no pueden ser modificados ni eliminados 🟢
- No se puede mover un bloque a sus propios hijos — detección de movimiento circular en `move-blocks` 🟢
- Orden entre siblings es lexicográfico generado vía `db-order/gen-key` — permite inserción sin reindexar toda la lista 🟢
- `:db/ident` es único globalmente en el schema de DataScript 🟢
- `block/uuid` tiene restricción `:db/unique` = `:db.unique/identity` — no puede haber dos entidades con el mismo UUID 🟢
- `file/path` tiene restricción `:db/unique` = `:db.unique/identity` — no puede haber dos archivos con la misma ruta 🟢
- Las queries DSL se parsean recursivamente: `and`, `or`, `not` se traducen a cláusulas Datalog equivalentes 🟢
- Time helpers (`today`, `-7d`, `+1w`, etc.) se resuelven en `pre-transform` antes del parseo principal 🟢
- Queries reactivas mantienen un cache por clave (`k`) y se invalidan selectivamente al modificar datos relacionados 🟢
- Las transacciones asíncronas se ejecutan en worker thread para no bloquear el event loop de UI 🟢
- Las funciones async (`<get-block`, `<get-blocks`, etc.) son wrappers de promesa sobre las funciones síncronas 🟡

## Fluxo Principal

### Inicialización de base de datos
1. Al abrir un grafo, se llama a `restore-graph!` con la URL del repositorio
2. `restore-graph!` determina el backend de persistencia (IndexedDB, SQLite, archivos)
3. Carga los datos persistentes desde el backend a memoria
4. `start!` crea una conexión DataScript con los datos restaurados:
   a. Si no hay datos previos, inicia con schema vacío (`logseq.db/create-conn`)
   b. Registra listeners de transacciones para sincronizar cambios
5. La conexión se almacena en `conn-state` indexada por URL de repositorio
6. `get-db` queda disponible para obtener snapshots inmutables de la DB actual

### Pipeline de query DSL
1. Usuario o componente invoca `query(repo, "(and (task TODO) (priority A))", opts)`
2. `pre-transform` resuelve time helpers:
   - `today` → timestamp del día actual
   - `-7d` → timestamp de hace 7 días
   - `+1w` → timestamp de dentro de 1 semana
3. `parse` analiza el string de query:
   a. Detecta si es una query DSL (paréntesis balanceados) o referencia directa `[[page]]`
   b. Extrae operadores y sus argumentos
   c. Si es referencia directa, genera query Datalog para blocks de esa página
4. `build-query` traduce recursivamente la expresión DSL a Datalog:
   a. `(and q1 q2)` → `[:find ?b :where (q1-datalog) (q2-datalog)]`
   b. `(or q1 q2)` → reglas Datalog con `or-rule` para cada rama
   c. `(not q)` → `(not-join [vars] (q-datalog))`
   d. `(task TODO DONE)` → filtro sobre `:block/properties` con `:logseq.property/status`
   e. `(priority A)` → filtro sobre `:block/properties` con `:logseq.priority/a`
   f. `(page "Name")` → join con entidad de página por `:block/name`
   g. `(between start end)` → filtro de rango sobre `:block/journal-day`
   h. `(property key value)` → lookup en `:block/properties`
   i. `(sample n)` → se añade al resultado para aplicar random sampling post-query
5. Se ejecuta `d/q` con la query Datalog generada contra la DB actual
6. Si hay `sample`, se aplica random shuffle y se toman N resultados
7. Resultados se retornan como lista de entidades DataScript

### Query reactiva (auto-refresh)
1. Componente Rum invoca `q(repo, :my-query-key, opts, query-datalog, inputs*)`
2. Se verifica si existe entrada en cache con la clave `[:my-query-key inputs]`
3. Si no existe cache:
   a. Se crea `QueryCacheEntry` con query, inputs y transform-fn
   b. Se ejecuta la query Datalog contra la DB actual
   c. El resultado se almacena en un átomo
4. Si existe cache:
   a. Se retorna el átomo con el resultado cacheado
   b. NO se re-ejecuta la query (los datos no han cambiado)
5. El componente usa `rum/react` sobre el átomo retornado
6. Cuando se ejecuta cualquier transacción que afecte datos relevantes:
   a. `refresh!` se invoca con las claves afectadas
   b. Se invalidan las entradas de cache correspondientes
   c. Se re-ejecutan las queries afectadas
   d. Los átomos se actualizan con nuevos resultados
   e. Rum re-renderiza los componentes suscritos a esos átomos
7. `run-custom-queries-when-idle!` ejecuta queries pendientes cuando el event loop está idle

### Navegación de bloques (get-next/get-prev)
1. `get-next(db, block-id, opts)` se invoca para obtener el siguiente bloque visible
2. Se obtiene el bloque actual por `db-id` vía `d/entity`
3. Se consulta `:block/parent` y `:block/order` del bloque actual
4. Se busca el siguiente sibling con `:block/order` > orden actual y mismo `:block/parent`
5. Si el bloque actual está colapsado y `collapse?` es true (default):
   a. Se salta recursivamente todos sus hijos
   b. Se busca el siguiente bloque después del último hijo
6. Si no hay más siblings en el mismo nivel:
   a. Se sube al `:block/parent`
   b. Se busca el siguiente sibling del padre (tío)
   c. Se repite hasta encontrar un bloque o llegar a la raíz de la página
7. `get-prev` opera de forma inversa (anterior sibling o último hijo del sibling anterior)
8. `get-block-deep-last-open-child-id` obtiene el último hijo visible en profundidad

### Transacción con operaciones del outliner
1. `apply-outliner-ops(conn, ops, opts)` recibe operaciones del outliner
2. Para cada operación en `ops`:
   a. Se identifica el tipo (`:save-block`, `:insert-blocks`, `:delete-blocks`, `:move-blocks`, etc.)
   b. Se validan las reglas de negocio (no built-in, no circular, UUID inmutable)
   c. Se construye el `tx-data` correspondiente
3. Si hay operaciones de inserción con referencias entre bloques nuevos:
   a. Se asignan IDs temporales (`assign-temp-id`) para resolver referencias cruzadas
   b. `d/transact!` resuelve los IDs temporales a IDs reales
4. Se ejecuta `d/transact!` con el batch completo de `tx-data`
5. Tras la transacción:
   a. Se emiten hooks de plugin (`hook:db-tx`, `hook:block-changes`)
   b. Se actualiza el índice de búsqueda si es necesario
   c. Se refrescan las queries reactivas afectadas

## Fluxos Alternativos
- **[Query DSL con sintaxis inválida]:** Si `parse` encuentra paréntesis no balanceados u operadores desconocidos, se lanza excepción con mensaje descriptivo y la query no se ejecuta 🟡
- **[Página no encontrada en get-page]:** Si `get-page` no encuentra la página por nombre, ID o UUID, retorna `nil` sin lanzar excepción — el caller es responsable de manejar el nil 🟢
- **[Transacción en worker thread (async)]:** Si la transacción es pesada, se envía al worker vía `transact` que retorna una `Promise<Map>`; el caller puede encadenar `.then()` para actuar tras completar 🟢
- **[Cache hit en query reactiva con inputs cambiados]:** Si los inputs de una query cacheada cambian, se invalida la entrada anterior y se crea una nueva con los nuevos inputs — no se reutiliza el resultado anterior 🟢
- **[get-next en el último bloque de la página]:** Si no hay siguiente bloque (fin de página), `get-next` retorna `nil` — el caller debe manejar el caso de fin de documento 🟢
- **[Delete de grafo con datos no persistidos]:** `delete-graph!` primero vacía la DB en memoria, luego elimina los archivos/datos del backend de persistencia — si falla el backend, la DB en memoria ya está limpia 🟡
- **[Sample con N > total de resultados]:** Si `(sample 10)` se aplica a una query que retorna solo 3 resultados, se retornan los 3 sin error — sample no fuerza mínimo de resultados 🟡

## Dependências
- `datascript.core` — base de datos inmutable: `d/entity`, `d/q`, `d/pull`, `d/pull-many`, `d/transact!`, `d/datoms`, `d/create-conn`
- `logseq.db` (deps/db) — schema de base de datos (`ldb/get-page`, `ldb/page-exists?`, `ldb/transact!`, funciones de schema)
- `frontend.state` — estado global de la aplicación
- `frontend.db.conn-state` — gestión atómica de conexiones por repositorio
- `logseq.outliner.op` — definiciones de operaciones del outliner
- `frontend.db.async.util` — utilidades para ejecución en worker thread
- `lambdaisland.glogi` — logging estructurado
- `promesa.core` — promesas al estilo Clojure(Script)
- `clojure.core.async` — canales para event loop y debouncing
- `cljs-time.core` — manipulación de fechas y timestamps
- `cljs-bean.core` — conversión entre objetos JS y mapas Clojure(Script)

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Consistência | UUID de bloque immutable — validado en cada save-block | `deps/outliner/src/logseq/outliner/core.cljs:316-321` | 🟢 |
| Consistência | `:db/ident` y `block/uuid` con unique identity en schema | `deps/db/src/logseq/db/frontend/schema.cljs:58-61` | 🟢 |
| Performance | Queries reactivas con cache por clave para evitar recálculos innecesarios | `src/main/frontend/db/react.cljs` — `q` y `QueryCacheEntry` | 🟢 |
| Performance | Transacciones asíncronas en worker thread para no bloquear UI | `src/main/frontend/db/transact.cljs` — `transact` | 🟢 |
| Performance | Batched block fetching con queue y timer flush | `src/main/frontend/db/async.cljs` — patrón de batch | 🟢 |
| Performance | Query result transformation vía `sci/eval-string` para post-procesado eficiente | `src/main/frontend/db/query_react.cljs` — `custom-query-result-transform` | 🟡 |
| Escalabilidade | Múltiples backends de persistencia (IndexedDB, SQLite, archivos) | `src/main/frontend/db/persist.cljs` — backends listados | 🟢 |
| Escalabilidade | Conexiones DataScript independientes por grafo (aislamiento de datos) | `src/main/frontend/db/conn_state.cljs` — mapa por URL | 🟢 |
| Disponibilidade | Restauración de grafos desde backup/persistencia tras error | `src/main/frontend/db/restore.cljs` — `restore-graph!` | 🟡 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

### Cenário: Query DSL con operadores anidados
```gherkin
Dado un grafo con 10 bloques marcados como TODO y prioridad A
  Y 5 bloques marcados como TODO con prioridad B
  Y 3 bloques DONE con prioridad A
  Y la query DSL: "(and (task TODO) (priority A))"
Cuando se llama a `query(repo, "(and (task TODO) (priority A))", {})`
Então `pre-transform` resuelve cualquier time helper en la query
  Y `parse` retorna un mapa con `:query` Datalog y `:rules`
  Y `build-query` genera cláusulas Datalog que intersectan task=TODO y priority=A
  Y el resultado contiene exactamente 10 bloques
  Y ningún bloque tiene priority B ni estado DONE
```

### Cenário: Búsqueda de página por nombre con page-exists?
```gherkin
Dado un grafo con una página llamada "Projects"
  Y no existe página llamada "Nonexistent"
Cuando se llama a `(page-exists? "projects" nil)`
Então retorna `true` (búsqueda case-insensitive)
Cuando se llama a `(page-exists? "nonexistent" nil)`
Então retorna `false`
```

### Cenário: Navegación de bloques con colapso
```gherkin
Dado una página con bloques en orden: A (con hijos A1, A2), B (con hijo B1), C
  Y el bloque A está colapsado (`:block/collapsed?` = true)
  Y los bloques B y C están expandidos
Cuando se llama a `(get-next db (:db/id A) {:collapse? true})`
Então retorna el bloque B (salta A1 y A2 porque A está colapsado)
Cuando se llama a `(get-next db (:db/id B) {:collapse? true})`
Então retorna el bloque B1 (hijo de B, porque B está expandido)
Cuando se llama a `(get-next db (:db/id C) {:collapse? true})`
Então retorna `nil` (fin de la página)
```

### Cenário: Transacción async con callback
```gherkin
Dado una conexión DataScript activa
  Y `tx-data` con operaciones de inserción de 3 bloques nuevos
Cuando se llama a `(transact worker-transact repo tx-data {:source "test"})`
Então se retorna una Promise
  Y la transacción se ejecuta en worker thread (no bloquea el thread principal)
  Y al resolverse la Promise, el resultado contiene `:tx-report` con los IDs asignados
  Y los 3 bloques son consultables vía `get-block-by-uuid` tras resolverse la Promise
```

### Cenário: Query reactiva con invalidación automática
```gherkin
Dado un componente Rum suscrito a `(q repo :page-blocks-key {} query-datalog [page-id])`
  Y el resultado inicial contiene 5 bloques
Cuando se ejecuta una transacción que añade un nuevo bloque a la misma página
  Y se llama a `(refresh! repo [:page-blocks-key])`
Então la entrada en cache se invalida
  Y la query se re-ejecuta automáticamente
  Y el átomo de resultado se actualiza con 6 bloques
  Y el componente Rum se re-renderiza con el nuevo resultado
```

### Cenário: Resolución de time helpers en query DSL
```gherkin
Dado que hoy es 2026-05-02
  Y un grafo con journals de los últimos 10 días
  Y la query DSL: "(between -3d today)"
Cuando se llama a `query(repo, "(between -3d today)", {})`
Então `pre-transform` convierte `-3d` a `20260429` y `today` a `20260502`
  Y `build-query` genera filtro Datalog: `journal-day >= 20260429 AND journal-day <= 20260502`
  Y el resultado contiene bloques de los journals del 29 abril al 2 mayo
  Y no contiene bloques de journals anteriores al 29 abril
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| Conexión DataScript y acceso a DB | Must | Sin acceso a datos, nada funciona — base de toda la aplicación |
| get-block-by-uuid / get-page | Must | Funciones más utilizadas en todo el código base |
| Transacciones batch (transact!) | Must | Cada edición, inserción o eliminación es una transacción |
| Query DSL con operadores | Must | Motor de queries principal para vistas y búsquedas |
| Navegación de bloques (get-next/get-prev) | Must | Requerida para navegación por teclado (flechas) |
| has-children? / get-block-parent | Must | Esencial para renderizado de outliner |
| Queries reactivas (q) | Should | Mejora significativa de UX pero las queries directas funcionan |
| Refresh automático de queries | Should | Importante para consistencia visual pero no bloqueante |
| Transacciones async en worker | Should | Mejora performance percibida pero las síncronas funcionan |
| Funciones async (<get-block, <get-blocks) | Should | Conveniencia para código async pero wrappers de las síncronas |
| Persistencia multi-backend | Should | La app funciona con un solo backend; multi-backend es escalabilidad |
| get-all-classes / get-all-properties | Should | Funcionalidad de descubribilidad pero no core |
| run-custom-queries-when-idle! | Could | Optimización de rendimiento, no funcionalidad visible |
| sort-by-order-recursive | Could | Operación de utilidad, raramente invocada directamente |
| get-class-objects | Could | Solo relevante para vistas de base de datos tipo clase |
| custom-query-result-transform | Could | Transformación avanzada de resultados, uso esporádico |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Cenários de Borda

### 1. Query DSL con anidación profunda (más de 10 niveles)
**Situação:** Un usuario escribe una query DSL con 15 niveles de anidación: `(and (or (and ... (or ...)) (not (and ...))) ...)`.
**Comportamento esperado:**
- `build-query` recorre recursivamente hasta el nivel máximo (no hay límite hardcoded)
- La query Datalog generada puede ser extensa pero DataScript la ejecuta correctamente
- Si la query resultante excede los límites de DataScript (memoria), se lanza error descriptivo
- No hay stack overflow porque la recursión es tail-recursive en Clojure(Script)
- Se recomienda no anidar más de 5-6 niveles para legibilidad

### 2. Cache de queries reactivas con alta cardinalidad de inputs
**Situação:** 50 componentes Rum concurrentes ejecutan la misma query reactiva pero con diferentes `page-id` como input.
**Comportamento esperado:**
- Cada combinación única de `[query-key inputs]` genera una entrada de cache separada
- 50 componentes con 50 page-ids distintos = 50 entradas de cache independientes
- Al modificar un bloque de una página, solo se invalida la entrada correspondiente a ese page-id
- Las otras 49 entradas permanecen en cache sin recalcular
- Si la memoria de cache crece excesivamente, entradas LRU se evictan automáticamente

### 3. Transacción con 10,000+ bloques en batch único
**Situação:** Importación masiva de datos (ej: migración desde otro sistema) que genera 10,000+ `tx-data` en una sola transacción.
**Comportamento esperado:**
- DataScript procesa la transacción como un batch atómico: todo o nada
- La transacción es bloqueante para esa conexión pero otras conexiones (otros grafos) no se afectan
- `transact` envía la transacción al worker thread para no bloquear UI
- Tras completar, los índices de búsqueda se reconstruyen incrementalmente (no full rebuild)
- Si falla a mitad de camino, se hace rollback completo (atomicidad DataScript)

### 4. Restauración de grafo con datos corruptos en persistencia
**Situação:** El backend de persistencia (IndexedDB/SQLite) contiene datos corruptos para un grafo debido a un cierre inesperado previo.
**Comportamento esperado:**
- `restore-graph!` intenta cargar los datos desde el backend
- Si la deserialización falla, se captura la excepción y se loggea el error
- Se intenta cargar desde un backup automático si existe
- Si no hay backup, se inicia un grafo vacío con el schema base
- Se notifica al usuario del problema y se sugiere verificar backups manuales
- No se pierden los archivos Markdown/Org originales (fuente de verdad)

### 5. Query DSL con time helper en zona horaria distinta a UTC
**Situação:** Usuario en zona horaria GMT+8 escribe `(between today today)` esperando ver los bloques de "hoy" en su zona horaria local.
**Comportamento esperado:**
- `pre-transform` resuelve `today` usando `cljs-time` que respeta la zona horaria del sistema
- `journal-day` se almacena como entero YYYYMMDD basado en la fecha local del archivo journal
- La comparación `journal-day >= YYYYMMDD AND journal-day <= YYYYMMDD` funciona correctamente
- Un journal creado a las 23:00 UTC del 1 de mayo (07:00 del 2 de mayo en GMT+8) tendrá `journal-day = 20260502`
- La query `(between today today)` en GMT+8 a las 07:00 del 2 de mayo encontrará ese journal correctamente

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `src/main/frontend/db/conn.cljs` | `get-db` | 🟢 |
| `src/main/frontend/db/conn.cljs` | `transact!` | 🟢 |
| `src/main/frontend/db/conn.cljs` | `start!` | 🟢 |
| `src/main/frontend/db/conn_state.cljs` | Connection state management | 🟢 |
| `src/main/frontend/db/transact.cljs` | `transact` | 🟢 |
| `src/main/frontend/db/transact.cljs` | `apply-outliner-ops` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-by-uuid` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-page` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-journal-page` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-journal-page-by-day` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-today-journal-page` | 🟢 |
| `src/main/frontend/db/model.cljs` | `page-exists?` | 🟢 |
| `src/main/frontend/db/model.cljs` | `journal-page?` | 🟢 |
| `src/main/frontend/db/model.cljs` | `today-journal-page?` | 🟢 |
| `src/main/frontend/db/model.cljs` | `has-children?` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-parent` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-parents` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-parents-v2` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-deep-last-open-child-id` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-next` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-prev` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-page-blocks-no-cache` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-page-blocks-count` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-latest-journals` | 🟢 |
| `src/main/frontend/db/model.cljs` | `sort-by-order-recursive` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-page` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-immediate-children` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-block-and-children` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-all-classes` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-all-properties` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-structured-children` | 🟢 |
| `src/main/frontend/db/model.cljs` | `get-class-objects` | 🟢 |
| `src/main/frontend/db/query_dsl.cljs` | `query` | 🟢 |
| `src/main/frontend/db/query_dsl.cljs` | `parse` | 🟢 |
| `src/main/frontend/db/query_dsl.cljs` | `build-query` | 🟢 |
| `src/main/frontend/db/query_dsl.cljs` | `pre-transform` | 🟢 |
| `src/main/frontend/db/query_dsl.cljs` | `custom-query` | 🟢 |
| `src/main/frontend/db/query_react.cljs` | `react-query` | 🟢 |
| `src/main/frontend/db/query_react.cljs` | `custom-query-result-transform` | 🟢 |
| `src/main/frontend/db/query_custom.cljs` | Custom query rules | 🟡 |
| `src/main/frontend/db/react.cljs` | `q` | 🟢 |
| `src/main/frontend/db/react.cljs` | `refresh!` | 🟢 |
| `src/main/frontend/db/react.cljs` | `refresh-affected-queries!` | 🟢 |
| `src/main/frontend/db/react.cljs` | `run-custom-queries-when-idle!` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-block` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-blocks` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-block-parents` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-block-refs` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-block-refs-count` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-date-scheduled-or-deadlines` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-files` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-tag-objects` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<task-spent-time` | 🟢 |
| `src/main/frontend/db/async.cljs` | `<get-asset-with-checksum` | 🟢 |
| `src/main/frontend/db/async/util.cljs` | Async worker utilities | 🟡 |
| `src/main/frontend/db/persist.cljs` | `get-all-graphs` | 🟢 |
| `src/main/frontend/db/persist.cljs` | `delete-graph!` | 🟢 |
| `src/main/frontend/db/restore.cljs` | `restore-graph!` | 🟢 |
| `src/main/frontend/db/utils.cljs` | `entity` | 🟢 |
| `src/main/frontend/db/utils.cljs` | `pull` | 🟢 |
| `src/main/frontend/db/utils.cljs` | `pull-many` | 🟢 |
| `src/main/frontend/db/utils.cljs` | `q` (util) | 🟢 |
| `src/main/frontend/db/debug.cljs` | Debug utilities | 🟡 |
