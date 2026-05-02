# frontend/handler

## Visão Geral
Sistema centralizado de manejo de eventos de Logseq. Implementa un event loop asíncrono con `core.async` y despacho vía `multimethod`. Es el orquestador principal que recibe acciones del usuario (teclado, clics, drags), eventos del sistema (RTC, sync, DB worker) y hooks de plugins, y los traduce en operaciones concretas sobre DataScript, el sistema de archivos y el estado global. Cada evento se procesa en el channel asíncrono, se despacha al handler correspondiente y sus errores se capturan y reportan a Sentry.

## Responsabilidades
- Inicializar y mantener el event loop principal (`run!`) con `async/go-loop`
- Despachar eventos por tipo mediante `multimethod` (`handle`) con `defmethod` por tipo de evento
- Gestionar operaciones del editor: guardar, insertar, eliminar, formatear bloques
- Gestionar operaciones de página: crear, eliminar, renombrar, crear favoritos
- Gestionar operaciones de bloque: editar, seleccionar, manejar eventos táctiles
- Gestionar repositorios/grafos: crear, eliminar, restaurar, cambiar entre grafos
- Coordinar la búsqueda: construcción del índice, programación con debounce (5s idle), agregación de resultados
- Manejar eventos de UI: modales, notificaciones, re-renderizado, exportación
- Manejar eventos RTC: sincronización de estado, presencia, descarga de grafos remotos
- Invocar hooks de plugins tras transacciones (`hook:db-tx`, `hook:block-changes`)
- Capturar errores de eventos y enviarlos a Sentry + logger

## Interface

### Event Loop Core (`events.cljs`)

```clojure
;; Inicializar el event loop
(run!)
;; Retorna: chan — canal de core.async

;; Despachar un evento
(state/pub-event! [:event-type payload])
;; event-type: Keyword — tipo de evento
;; payload:    Any     — datos del evento
;; Retorna: nil (publica en channel)

;; Multimethod principal
(defmulti handle first)
;; Despacha por el primer elemento del vector (tipo de evento)

;; Ejemplo de handler:
(defmethod handle :event/type [[_ payload]]
  ;; lógica del handler
  )
```

### Tipos de eventos principales y sus payloads

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `:graph/switch` | `graph opts` | Cambiar a otro grafo/repositorio |
| `:graph/open-new-window` | `target-repo` | Abrir grafo en nueva ventana |
| `:graph/ready` | `repo` | Grafo listo para mostrar en UI |
| `:graph/restored` | `graph` | Grafo restaurado tras window reload |
| `:graph/save-db-to-disk` | `opts` | Guardar DB a persistencia |
| `:graph/sync-context` | `-` | Sincronizar contexto al worker |
| `:page/create` | `page-name opts` | Crear nueva página |
| `:page/deleted` | `page-name tx-meta` | Página eliminada |
| `:page/renamed` | `repo data` | Página renombrada |
| `:page/create-today-journal` | `-` | Crear journal del día actual |
| `:editor/save-current-block` | `-` | Guardar bloque en edición |
| `:editor/insert-new-block` | `state right-sibling` | Insertar nuevo bloque |
| `:editor/delete-block` | `repo` | Eliminar bloque(s) |
| `:editor/set-heading` | `block heading` | Establecer nivel de heading |
| `:editor/upsert-type-block` | `{:keys [block type lang]}` | Cambiar tipo de bloque |
| `:editor/quick-capture` | `args` | Captura rápida (hotkey) |
| `:editor/toggle-own-number-list` | `-` | Alternar lista numerada |
| `:db/sync-changes` | `data` | Cambios sincronizados desde DB |
| `:db/export-sqlite` | `-` | Exportar DB a SQLite |
| `:rtc/sync-state` | `state` | Estado de sincronización RTC |
| `:rtc/presence-update` | `presence-data` | Actualización de presencia |
| `:rtc/download-remote-graph` | `graph-name uuid schema e2ee?` | Descargar grafo remoto |
| `:search/rebuild` | `repo` | Reconstruir índice de búsqueda |
| `:ui/re-render-root` | `-` | Forzar re-renderizado completo |
| `:notification/show` | `{:keys [content status]}` | Mostrar notificación |
| `:plugin/hook-db-tx` | `{:blocks tx-data}` | Notificar plugins de transacción |

### Estados y entidades del handler

**EventPayload:**
```clojure
{:event-type Keyword  ;; tipo de evento (:graph/switch, :page/create, etc.)
 :data       Any}     ;; datos del evento (varía por tipo)
```

**EditorState (handler):**
```clojure
{:block/uuid              UUID      ;; bloque en edición
 :block/title             String?   ;; contenido actual
 :block/format            Keyword   ;; :markdown | :org
 :editor/cursor-range     [Int Int]? ;; posición del cursor
 :block.editing/direction Keyword?  ;; :up | :down | :max
 :block.editing/pos       Int?}     ;; posición de edición
```

**OutlinerOp:**
```clojure
{:outliner-op        Keyword  ;; :save-block | :insert-blocks | :delete-blocks | :move-blocks
 :source-outliner-op Keyword?} ;; operación origen (para tracking)
```

## Regras de Negócio
- Todos los eventos se procesan vía `core.async` channel con `async/go-loop` — ejecución asíncrona no bloqueante 🟢
- Eventos se despachan vía `multimethod` con `defmulti handle first` — el primer elemento del vector determina el handler 🟢
- Errores en eventos se capturan con `try/catch`, se loggean y se envían a Sentry vía `capture-error` — los errores no rompen el event loop 🟢
- El índice de búsqueda se construye solo cuando el usuario está idle por 5 segundos (`schedule-search-index-build!` con timeout) 🟢
- Bloques reciclados (en papelera) son read-only — `edit-block!` muestra notificación de solo lectura y no permite edición 🟢
- Tags/clases privados (con prefijo `_` o visibilidad restringida) no pueden asignarse a páginas — validado en `<create!` 🟢
- Al cambiar de grafo, se cancela el backup automático pendiente, se limpian queries async, y se restaura el nuevo grafo desde persistencia 🟢
- Las operaciones del editor (save, insert, delete) se traducen a operaciones del outliner y se ejecutan vía `ui-outliner-tx/transact!` 🟢
- Las transacciones del outliner disparan el pipeline de hooks que notifica a los plugins registrados 🟢
- La creación de página valida que el nombre no contenga caracteres prohibidos ni sea tag privado 🟢
- Al crear un journal, si ya existe para ese día, no se duplica — se retorna la página existente 🟢

## Fluxo Principal

### Event loop — recepción y despacho
1. `run!` es invocado durante la inicialización de la aplicación
2. Crea o reutiliza un canal de `core.async` (`get-events-chan`)
3. Inicia `async/go-loop` que:
   a. Bloquea en `async/<!` esperando el próximo evento del canal
   b. Al recibir un evento `[event-type payload]`, llama a `(handle [event-type payload])`
   c. `handle` es un multimethod que despacha por `event-type`
   d. El `defmethod` correspondiente ejecuta la lógica del handler
   e. El resultado se envuelve en `p/then` (promesa) para manejo asíncrono
   f. Si ocurre error: `p/catch` loggea el error y llama a `capture-error` para Sentry
   g. Vuelve al paso (a) para el siguiente evento
4. El event loop nunca termina — es un bucle infinito que solo se detiene al cerrar la app

### Edición y guardado de bloque
1. Usuario modifica un bloque en el editor y pulsa Enter (o hace clic fuera)
2. `editor/save-current-block!` se invoca:
   a. Obtiene el estado actual del editor: `editor/block`, `editor/content`, `editor/action`
   b. Compara el contenido actual con el original para detectar cambios
   c. Si no hay cambios, sale sin hacer nada (optimización)
3. Si hay cambios:
   a. `wrap-parse-block` parsesa el contenido y resuelve referencias ([[page]], ((block)), #tag)
   b. Construye `OutlinerOp` de tipo `:save-block` con los datos del bloque
   c. Envía la operación a `ui-outliner-tx/transact!`
   d. `transact!` ejecuta la transacción en DataScript vía `db/transact!`
   e. Tras la transacción, `pipeline/invoke-hooks` notifica a los plugins
4. Si el usuario pulsó Enter (no clic fuera):
   a. `insert-new-block!` crea un nuevo bloque vacío debajo del editado
   b. El nuevo bloque recibe el foco del editor automáticamente
5. El estado global se actualiza y los componentes Rum se re-renderizan reactivamente

### Inserción de nuevo bloque
1. `insert-new-block!` se invoca (normalmente por Enter en el editor)
2. Recibe `state` (estado del editor) y `right-sibling` (si debe insertar como sibling o child)
3. `compute-fst-snd-block-text` divide el texto si el cursor está en medio del bloque
4. Calcula el `target-block` y determina si es `sibling?` o child
5. `insert-new-block-aux!` construye la operación del outliner:
   a. Crea un bloque vacío con UUID nuevo
   b. Asigna `:block/parent` y `:block/page` según el contexto
   c. Calcula `:block/order` entre siblings existentes
6. Envía `OutlinerOp` de tipo `:insert-blocks` a `ui-outliner-tx/transact!`
7. La transacción se aplica en DataScript y el pipeline de hooks se invoca

### Eliminación de bloque
1. `delete-block!` se invoca (normalmente por Backspace en bloque vacío, o Delete con selección)
2. Obtiene los bloques actuales del estado del editor
3. Si hay múltiples bloques seleccionados:
   a. Obtiene todos los bloques en la selección vía `util/get-selected-blocks`
   b. Los elimina en batch
4. Si es un solo bloque:
   a. `delete-block-inner!` construye `OutlinerOp` de tipo `:delete-blocks`
   b. El outliner maneja orfandad: páginas sin bloques → recycle, refs huérfanas → cleanup
5. Envía a `ui-outliner-tx/transact!` para aplicar la transacción
6. Si el bloque eliminado era el último de la página, la página se mueve a recycle

### Cambio de grafo
1. Se publica evento `:graph/switch` con el grafo destino
2. `handle` despacha al handler de `:graph/switch`:
   a. Cancela backup automático pendiente (`export/cancel-db-backup!`)
   b. Limpia estado de queries asíncronas (`state/set-state! :db/async-queries {}`)
   c. Refresca queries reactivas (`st/refresh!`)
3. `graph-switch-on-persisted` se invoca:
   a. `repo-handler/restore-and-setup-repo!` restaura la DB del grafo desde persistencia
   b. `db-restore/restore-graph!` carga datos desde IndexedDB/SQLite/archivos
   c. `repo-config-handler/restore-repo-config!` restaura la configuración del repositorio
   d. Si `global-config-enabled?`, restaura configuración global
4. Post-restauración:
   a. `ui-handler/add-style-if-exists!` aplica estilos CSS personalizados
   b. `page-handler/init-commands!` inicializa comandos slash
   c. `route-handler/redirect-to-home!` redirige a la página de inicio
   d. `graph-handler/settle-metadata-to-local!` sincroniza metadata local
5. Si hay descarga RTC pendiente (`rtc-download?`):
   a. `repo-handler/refresh-repos!` actualiza lista de repositorios remotos
6. `schedule-search-index-build!` programa la reconstrucción del índice
7. `export/backup-db-graph` inicia backup del grafo actual

### Creación de página
1. Se publica evento `:page/create` con `page-name` y `opts`
2. `handle` despacha al handler de `:page/create`:
   a. Si `today journal?` es true → `page-handler/create-today-journal!`
   b. Si no → `page-common-handler/<create!`
3. Para journal:
   a. `date/today` obtiene la fecha actual
   b. `state/set-today!` actualiza el estado
   c. `db/get-today-journal-page` verifica si ya existe
   d. Si no existe, `ui-outliner-tx/transact!` con `outliner-op/create-page!`
   e. `plugin-handler/hook-plugin-app :today-journal-created` notifica plugins
4. Para página normal (`<create!`):
   a. `wrap-tags` extrae y normaliza tags si el nombre contiene `#`
   b. `db-editor-handler/wrap-parse-block` parsesa el bloque de la nueva página
   c. Si tiene tags, valida que no sean privados
   d. Si la validación falla, `notification/show!` muestra error
   e. Si es válido, `ui-outliner-tx/transact!` crea la página
   f. `route-handler/redirect-to-page!` redirige a la nueva página

### Sistema de búsqueda — construcción del índice
1. `schedule-search-index-build!` se invoca tras restaurar un grafo o tras cambios
2. Espera hasta que el estado `input-idle?` sea true (5 segundos sin input del usuario)
3. Si el usuario sigue escribiendo, se cancela y reprograma
4. Cuando idle, invoca `<build-search-index!>`:
   a. Envía trabajo al db-worker thread vía `thread-api/search-build-blocks-indice`
   b. El worker indexa todos los bloques del grafo para búsqueda full-text
5. Si la construcción falla:
   a. Se loggea error en consola
   b. Se reprograma reintento en 5 segundos
6. Si es exitoso, se loggea info y el índice queda disponible para búsquedas

## Fluxos Alternativos
- **[Error en handler de evento]:** Si un `defmethod` lanza excepción, `p/catch` la captura, `capture-error` la envía a Sentry, y el event loop continúa con el siguiente evento sin interrupción 🟢
- **[Intento de editar bloque reciclado]:** `edit-block!` detecta `recycled?` en el bloque, muestra notificación "Cannot edit recycled block" y aborta la edición 🟢
- **[Creación de página con tag privado]:** `<create!` detecta tags con prefijo `_` o visibilidad restringida, muestra `notification/show!` con error y no crea la página 🟢
- **[Journal ya existente]:** Si `create-today-journal!` detecta que el journal ya existe vía `db/get-today-journal-page`, redirige a la página existente sin crear duplicado 🟢
- **[Guardado sin cambios]:** `save-current-block!` compara el contenido actual con el original; si son idénticos, retorna nil sin ejecutar transacción 🟢
- **[Graph switch con error de persistencia]:** Si `restore-graph!` falla (datos corruptos), se captura el error, se loggea, y se inicia un grafo vacío con schema base — no se pierde el event loop 🟡
- **[Búsqueda con índice no construido]:** Si `search` se invoca antes de que el índice esté listo, se retorna resultado vacío en lugar de error 🟡
- **[Evento RTC con conexión perdida]:** Si `:rtc/sync-state` llega con estado `offline` o `error`, se actualiza el estado de UI para mostrar indicador de desconexión sin interrumpir otras operaciones 🟡

## Dependências
- `frontend.db` — transacciones DataScript (`transact!`, `transact`), acceso a entidades (`entity`, `get-block-by-uuid`, `get-page`)
- `frontend.state` — estado global (`pub-event!`, `sub`, `set-state!`, `get-state`)
- `frontend.handler.editor` — lógica específica del editor (save, insert, delete, format, cycle-todo)
- `frontend.handler.page` — utilidades de página (create, delete, rename, favorite, journal)
- `frontend.handler.block` — operaciones de bloque (edit, select, touch events)
- `frontend.handler.ui` — gestión de UI (modales, notificaciones, temas, sidebars)
- `frontend.handler.repo` — gestión de repositorios (create, delete, restore, switch)
- `frontend.handler.search` — agregación de búsqueda (blocks + files)
- `frontend.handler.common.editor` — utilidades de editor compartidas (file/DB agnostic)
- `frontend.handler.common.page` — utilidades de página compartidas (validación de tags)
- `frontend.handler.db_based.editor` — operaciones de editor para grafos DB-based
- `frontend.handler.db_based.page` — operaciones de página para grafos DB-based
- `frontend.modules.outliner.op` — construcción de operaciones del outliner
- `frontend.modules.outliner.ui` — transacciones UI del outliner (`ui-outliner-tx/transact!`)
- `logseq.outliner.op` — definiciones core de operaciones del outliner
- `clojure.core.async` — canales para event loop asíncrono
- `promesa.core` — promesas para operaciones asíncronas (`p/do!`, `p/then`, `p/catch`)

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Resiliência | Errores en handlers no rompen el event loop — try/catch + Sentry + log | `src/main/frontend/handler/events.cljs:425-437` | 🟢 |
| Performance | Search index se construye solo cuando usuario está idle 5s — evita bloqueos durante escritura | `src/main/frontend/handler/events.cljs:73-91` | 🟢 |
| Performance | Guardado de bloque optimizado: no genera transacción si no hay cambios | `src/main/frontend/handler/editor.cljs` — `save-current-block!` | 🟢 |
| Segurança | Tags privados no pueden asignarse a páginas — validación en creación | `src/main/frontend/handler/common/page.cljs:76-84` | 🟢 |
| Segurança | Bloques reciclados son read-only — no permiten edición accidental | `src/main/frontend/handler/block.cljs:156-157` | 🟢 |
| Consistência | Graph switch cancela backups pendientes y limpia estado antes de cambiar | `src/main/frontend/handler/events.cljs` — `:graph/switch` handler | 🟢 |
| Extensibilidade | Plugin hooks se invocan tras cada transacción (`hook:db-tx`, `hook:block-changes`) | Pipeline de hooks en transacciones del outliner | 🟢 |
| Observabilidade | Errores de eventos se envían a Sentry para monitoreo | `src/main/frontend/handler/events.cljs:425-437` — `capture-error` | 🟢 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

### Cenário: Guardado de bloque con detección de cambios
```gherkin
Dado un bloque en edición con contenido "Original text"
  Y el usuario modifica el contenido a "Modified text"
Cuando se dispara `:editor/save-current-block` (Enter o clic fuera)
Então `save-current-block!` detecta que el contenido cambió
  Y `wrap-parse-block` parsesa el nuevo contenido resolviendo referencias
  Y se construye `OutlinerOp` de tipo `:save-block`
  Y `ui-outliner-tx/transact!` ejecuta la transacción en DataScript
  Y el bloque se persiste con `:block/title` = "Modified text"
  Y `pipeline/invoke-hooks` notifica a los plugins del cambio
```

### Cenário: Guardado sin cambios — no-op
```gherkin
Dado un bloque en edición con contenido "Same text"
  Y el usuario hace clic fuera sin modificar el contenido
Cuando se dispara `:editor/save-current-block`
Então `save-current-block!` compara el contenido actual con el original
  Y detecta que son idénticos
  Y retorna nil sin ejecutar ninguna transacción
  Y el bloque sale del modo edición normalmente
```

### Cenário: Intento de editar bloque reciclado
```gherkin
Dado un bloque que está en recycle (papelera)
  Y el usuario intenta hacer clic para editarlo
Cuando `edit-block!` se invoca con el bloque
Então se detecta que `recycled?` es true
  Y se muestra `notification/show!` con status "warning" y mensaje "Cannot edit recycled block"
  Y `state/set-editing!` NO se llama
  Y el bloque permanece en modo solo lectura
```

### Cenário: Creación de página con tag privado rechazada
```gherkin
Dado un usuario intenta crear una página con nombre "My Page #_private-tag"
Cuando se publica `:page/create` con `page-name` = "My Page #_private-tag"
Então `<create!` extrae los tags del nombre
  Y detecta que `_private-tag` es un tag privado
  Y la validación de tags falla
  Y se muestra `notification/show!` con error descriptivo
  Y no se crea la página en DataScript
  Y no se redirige a ninguna ruta
```

### Cenário: Creación de journal del día (sin duplicado)
```gherkin
Dado que hoy es 2026-05-02
  Y no existe journal para este día
Cuando se publica `:page/create-today-journal`
Então `date/today` obtiene la fecha actual
  Y `db/get-today-journal-page` retorna nil (no existe)
  Y `ui-outliner-tx/transact!` crea la página con `:block/journal-day` = 20260502
  Y se notifica a plugins vía `hook-plugin-app :today-journal-created`
  Y se redirige al nuevo journal

Dado que el journal del día ya existe
Cuando se publica `:page/create-today-journal` nuevamente
Então `db/get-today-journal-page` retorna la página existente
  Y no se crea duplicado
  Y se redirige al journal existente
```

### Cenário: Graph switch con persistencia y restauración
```gherkin
Dado el usuario está en el grafo "graph-a"
  Y quiere cambiar al grafo "graph-b"
Cuando se publica `:graph/switch` con `graph` = "graph-b"
Então se cancela el backup automático de "graph-a"
  Y se limpia el estado de queries asíncronas
  Y `restore-and-setup-repo!` restaura "graph-b" desde persistencia
  Y `restore-repo-config!` carga la configuración del repositorio
  Y se aplican estilos CSS personalizados si existen
  Y `redirect-to-home!` redirige a la página de inicio de "graph-b"
  Y `schedule-search-index-build!` programa la reconstrucción del índice
  Y se inicia backup de "graph-b" en segundo plano
```

### Cenário: Error en handler no rompe el event loop
```gherkin
Dado el event loop activo procesando eventos
  Y un handler `:event/broken` que lanza una excepción no controlada
Cuando se publica `:event/broken` con payload inválido
Então el `defmethod` lanza excepción
  Y `p/catch` captura la excepción
  Y `capture-error` envía el error a Sentry
  Y se loggea el error con detalles
  Y el event loop continúa procesando el siguiente evento
  Y los eventos posteriores se manejan normalmente
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| Event loop (run! + handle) | Must | Sin event loop no hay procesamiento de acciones del usuario |
| save-current-block! | Must | Cada edición de texto requiere guardado — operación más frecuente |
| insert-new-block! | Must | Crear nuevos bloques es operación core del outliner |
| delete-block! | Must | Eliminar bloques es parte del flujo normal de edición |
| Graph switch | Must | Cambio entre grafos es esencial para aplicación multi-repositorio |
| Page create | Must | Creación de páginas es operación fundamental |
| Manejo de errores con Sentry | Must | Crítico para diagnosticar fallos en producción |
| Search index scheduling | Must | Sin índice no hay búsqueda full-text |
| RTC events (sync-state, download) | Should | Importante para colaboración pero app funciona offline |
| Editor quick-capture | Should | Conveniencia de usuario, existe alternativa manual |
| Plugin hooks (hook-db-tx) | Should | Extensibilidad importante pero no bloqueante |
| Export SQLite | Could | Funcionalidad complementaria de exportación |
| toggle-own-number-list | Could | Formateo específico, baja frecuencia de uso |
| upsert-type-block | Could | Cambio de tipo de bloque poco frecuente |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Cenários de Borda

### 1. Ráfaga de eventos (event storm)
**Situação:** El usuario realiza 50+ acciones en rápida sucesión (ej: mantener pulsada la tecla de flecha para navegar bloques).
**Comportamento esperado:**
- Cada acción publica un evento en el channel de `core.async`
- Los eventos se encolan en el buffer del channel (por defecto `async/sliding-buffer`)
- Con sliding buffer, eventos nuevos desplazan a los antiguos si el buffer se llena
- El event loop procesa eventos uno a uno en orden FIFO
- Si la cola crece demasiado, se aplica backpressure: algunos eventos intermedios se descartan
- Los eventos críticos (save, delete) tienen prioridad implícita por orden de llegada
- La UI puede mostrar micro-retrasos pero no se congela

### 2. Graph switch durante transacción pendiente
**Situação:** El usuario cambia de grafo mientras hay una transacción `transact!` aún en progreso en el grafo actual.
**Comportamento esperado:**
- `:graph/switch` handler cancela explícitamente el backup pendiente (`cancel-db-backup!`)
- La transacción pendiente se completa o se descarta según su estado
- Las queries asíncronas pendientes se limpian (`state/set-state! :db/async-queries {}`)
- El cambio de grafo no espera a que la transacción termine
- Si la transacción pendiente era un save, el usuario podría perder ese cambio no guardado
- Se recomienda al usuario guardar antes de cambiar de grafo

### 3. Construcción de índice de búsqueda en grafo con 50,000+ bloques
**Situação:** Un grafo grande con más de 50,000 bloques requiere reconstrucción del índice de búsqueda.
**Comportamento esperado:**
- `<build-search-index!` envía el trabajo al db-worker thread — no bloquea el thread principal
- El worker procesa bloques en batches de N (ej: 500) para no saturar memoria
- Mientras se construye el índice, las búsquedas retornan resultados del índice anterior (si existe)
- Si el índice anterior no existe, las búsquedas retornan vacío hasta que el nuevo índice esté listo
- Si la construcción falla a mitad de camino, se reprograma reintento en 5 segundos
- El tiempo total de construcción es proporcional al número de bloques (~30-60s para 50K bloques)

### 4. Evento RTC durante modo offline
**Situação:** El usuario está offline y el sistema RTC intenta sincronizar cambios.
**Comportamento esperado:**
- `:rtc/sync-state` se publica con estado `offline`
- El handler actualiza el estado de UI para mostrar indicador de offline
- Las operaciones de edición local continúan normalmente
- Los cambios se acumulan en cola local pendiente de sincronización
- Cuando se recupera la conexión, `:rtc/sync-state` con estado `syncing` dispara la sincronización
- El merge de cambios remotos se maneja con resolución de conflictos estándar

### 5. Creación masiva de páginas vía plugin
**Situação:** Un plugin dispara 200 eventos `:page/create` en rápida sucesión (ej: importación).
**Comportamento esperado:**
- Cada evento se encola en el channel del event loop
- Las páginas se crean secuencialmente una por una
- Cada creación valida tags y nombres independientemente
- Si una página ya existe, se salta sin error (idempotencia por nombre)
- Las 200 páginas se crean en lote pero con transacciones individuales
- El rendimiento es limitado por la velocidad de DataScript (~50-100 páginas/segundo)
- La UI no se bloquea porque el event loop es asíncrono

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `src/main/frontend/handler/events.cljs` | `run!` | 🟢 |
| `src/main/frontend/handler/events.cljs` | `handle` (multimethod) | 🟢 |
| `src/main/frontend/handler/events.cljs` | `graph-switch-on-persisted` | 🟢 |
| `src/main/frontend/handler/events.cljs` | `schedule-search-index-build!` | 🟢 |
| `src/main/frontend/handler/events.cljs` | `<build-search-index!` | 🟢 |
| `src/main/frontend/handler/events/ui.cljs` | UI event handlers | 🟡 |
| `src/main/frontend/handler/events/rtc.cljs` | RTC event handlers | 🟡 |
| `src/main/frontend/handler/events/export.cljs` | Export event handlers | 🟡 |
| `src/main/frontend/handler/editor.cljs` | `save-current-block!` | 🟢 |
| `src/main/frontend/handler/editor.cljs` | `insert-new-block!` | 🟢 |
| `src/main/frontend/handler/editor.cljs` | `delete-block!` | 🟢 |
| `src/main/frontend/handler/editor.cljs` | `cycle-todo!` | 🟢 |
| `src/main/frontend/handler/editor.cljs` | `format-text!` | 🟢 |
| `src/main/frontend/handler/editor/lifecycle.cljs` | Editor lifecycle | 🟡 |
| `src/main/frontend/handler/block.cljs` | `edit-block!` | 🟢 |
| `src/main/frontend/handler/block.cljs` | `select-block!` | 🟢 |
| `src/main/frontend/handler/block.cljs` | Touch event handlers | 🟡 |
| `src/main/frontend/handler/page.cljs` | Page utilities | 🟢 |
| `src/main/frontend/handler/repo.cljs` | `delete-repo!` | 🟢 |
| `src/main/frontend/handler/repo.cljs` | `restore-and-setup-repo!` | 🟢 |
| `src/main/frontend/handler/repo.cljs` | `refresh-repos!` | 🟡 |
| `src/main/frontend/handler/search.cljs` | `search` | 🟢 |
| `src/main/frontend/handler/ui.cljs` | UI state management | 🟡 |
| `src/main/frontend/handler/common/editor.cljs` | Shared editor utilities | 🟢 |
| `src/main/frontend/handler/common/page.cljs` | `<create!` | 🟢 |
| `src/main/frontend/handler/db_based/editor.cljs` | `wrap-parse-block` | 🟢 |
| `src/main/frontend/handler/db_based/page.cljs` | DB-based page ops | 🟡 |
| `src/main/frontend/handler/db_based/property.cljs` | Property handling | 🟡 |
| `src/main/frontend/handler/db_based/sync.cljs` | DB sync operations | 🟡 |
| `src/main/frontend/handler/db_based/rtc_flows.cljs` | RTC flow orchestration | 🟡 |
