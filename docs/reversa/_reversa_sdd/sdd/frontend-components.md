# frontend/components

## Visão Geral
Módulo de componentes de interfaz de usuario de Logseq, construido sobre Rum (fork de React para ClojureScript). Contiene todos los elementos visuales de la aplicación: contenedor raíz, editor de texto, bloques, páginas, journals, sidebars, sistema de queries, propiedades, assets y navegación. Los componentes se comunican con DataScript mediante mixins reactivos y emiten eventos a través del handler system para modificar el estado.

## Responsabilidades
- Renderizar la estructura principal de la aplicación (container, sidebars, header)
- Renderizar páginas con sus bloques en vista de outliner
- Renderizar el editor de texto inline con auto-completado de comandos slash, búsqueda de páginas y bloques
- Renderizar assets (imágenes, PDFs, videos) con lazy loading y redimensionamiento
- Renderizar resultados de queries (DSL y custom) con agrupación por página
- Gestionar drag & drop de bloques con estado de arrastre y recálculo de orden
- Renderizar sidebars con navegación, favoritos, páginas recientes y referencias
- Renderizar el sistema de propiedades (diálogos, configuración, valores tipados)
- Renderizar vistas de grafos globales y por página con simulación de fuerzas
- Renderizar preview de páginas en hover con debounce de 1 segundo
- Renderizar breadcrumbs de navegación jerárquica

## Interface

### Mixins de componentes (Rum)

```clojure
;; Componente funcional reactivo
(rum/defc component-name < rum/reactive db-mixins/query
  [props]
  ...)

;; Componente con estado local
(rum/defcs component-name < rum/reactive
  {:init (fn [state] ...)
   :did-mount (fn [state] ...)}
  [state props]
  ...)

;; Acceso reactivo a átomos de estado
(rum/react state-atom)
(rum/local initial-value ::key)
```

### Props comunes por tipo de componente

**Container (`root-container`):**
```clojure
{:keys [route-match main-content]}
;; route-match: Map     — datos de ruta actual
;; main-content: Hiccup — contenido principal a renderizar
;; Retorna: Hiccup (árbol de DOM virtual)
```

**Block (`block`):**
```clojure
{:keys [block config sidebar? preview?]}
;; block:    Entity   — entidad DataScript del bloque
;; config:   Map      — configuración de renderizado
;; sidebar?: Boolean  — si se renderiza en sidebar
;; preview?: Boolean  — si es preview en hover
;; Retorna: Hiccup
```

**Editor (`box`):**
```clojure
{:keys [format block parent-block]}
;; format:        Keyword  — :markdown | :org
;; block:         Entity   — bloque a editar
;; parent-block:  Entity?  — bloque padre (para inserción)
;; id:            String   — ID único del elemento DOM
;; config:        Map      — configuración adicional
;; Retorna: Hiccup
```

**Page (`page-cp`):**
```clojure
;; Props:
{:keys [page-name repo sidebar? preview?]}
;; page-name: String  — nombre de la página
;; repo:      String  — URL del repositorio
;; sidebar?:  Boolean — si se renderiza en sidebar
;; preview?:  Boolean — si es preview
;; Retorna: Hiccup
```

### Entidades de UI (estructuras de estado local)

**BlockState:**
```clojure
{:block/uuid        UUID?       ;; UUID del bloque
 :block/title       String?     ;; título visible
 :block/page        {:db/id Int}? ;; página contenedora
 :block/format      Keyword?    ;; :markdown | :org
 :block/parent      {:db/id Int}? ;; bloque padre
 :alias             Map?        ;; alias de página
 :nlp-date?         Boolean?    ;; fecha NLP detectada
 :page?             Boolean?    ;; si es página raíz
 :block/collapsed?  Boolean?}   ;; si está colapsado
```

**EditorState:**
```clojure
{:editor/action          Atom    ;; acción actual del editor
 :editor/action-data     Any?    ;; datos de la acción
 :editor/cursor-range    [Int Int]? ;; rango del cursor [start end]
 :editor/content         Atom    ;; contenido del editor
 :editor/block           Atom    ;; bloque en edición
 :editor/editing?        Atom    ;; flag de edición activa
 :editor/in-composition? Boolean?} ;; composición IME activa
```

**DragState:**
```clojure
{*dragging?         Atom     ;; flag de arrastre activo
 *dragging-block    Atom     ;; bloque siendo arrastrado
 *dragging-over-block Atom   ;; bloque sobre el que se arrastra
 *drag-to           Atom}    ;; destino del drop
```

**QueryConfig:**
```clojure
{:dsl-query?      Boolean?   ;; si es query DSL
 :built-in-query? Boolean?   ;; si es query built-in
 :current-block   Entity?    ;; bloque actual (contexto)
 :view            Keyword|Fn ;; vista de resultados (table, list, etc)
 :collapsed?      Boolean?}  ;; estado de colapso
```

**AssetBlock:**
```clojure
{:block/uuid                       UUID     ;; UUID del bloque asset
 :block/title                      String?  ;; título/ruta del asset
 :logseq.property.asset/type       String   ;; "image" | "pdf" | "video" | "audio"
 :logseq.property.asset/width      Int?     ;; ancho en px
 :logseq.property.asset/height     Int?     ;; alto en px
 :logseq.property.asset/align      Keyword? ;; :left | :center | :right
 :logseq.property.asset/external-url String? ;; URL externa si es remoto}
```

## Regras de Negócio
- Bloques en un journal day no pueden ser editados directamente — se requiere confirmación explícita 🟢
- Drag & drop entre bloques requiere recalcular el orden sibling del destino (no se reordena toda la lista) 🟢
- Preview de página se activa tras 1 segundo de hover sobre una referencia de página (`[[page ref]]`) 🟢
- Assets remotos (URLs externas) se descargan y cachean localmente antes de mostrar 🟢
- El estado de colapso de queries se persiste en DataScript para sobrevivir a refrescos de página 🟢
- Left sidebar se cierra automáticamente en breakpoint mobile (viewport < threshold) 🟢
- Right sidebar tiene ancho mínimo de 320px y máximo de 70% del viewport 🟢
- Comandos slash (`/`) se filtran por contexto: los comandos de página no aparecen en contexto de bloque y viceversa 🟢
- Los bloques en vista de recycle (papelera) son read-only y no permiten edición inline 🟡
- Títulos de página con iconos se muestran con el icono junto al título en lugar de solo texto 🟡
- Bloques colapsados ocultan recursivamente todos sus hijos en la vista de outliner 🟢

## Fluxo Principal

### Renderizado de página completa
1. Router emite `route-match` con `{:route-name :page :path-params {:page-name "..."}}`
2. `container/main` recibe `route-match` y determina que es una página
3. `page/page-cp` recibe `page-name` y `repo`
4. `page-cp` consulta DataScript vía `db-mixins/query` para obtener la entidad de página por nombre
5. Si la página no existe, se muestra vista de "page not found" con opción de crear
6. Si existe, se renderiza:
   a. `db-page-title` — título con icono, tags, formato
   b. `page-blocks-cp` — lista de bloques top-level
   c. `reference/references` — referencias entrantes agrupadas
   d. `reference/unlinked-references` — referencias no vinculadas
   e. `scheduled-deadlines` — bloques con SCHEDULED/DEADLINE
   f. `today-queries` — queries del día
7. Cada bloque se renderiza vía `block/block`:
   a. Muestra marcador de tarea (NOW/LATER/TODO/DONE/CANCELLED) si aplica
   b. Muestra bullet de indentación con nivel jerárquico
   c. Muestra contenido del bloque (texto, referencias, timestamps)
   d. Si el bloque tiene hijos y NO está colapsado, los renderiza recursivamente
   e. Si el bloque tiene hijos y ESTÁ colapsado, muestra indicador de colapso con contador
   f. Si el bloque es un asset, renderiza `asset-container`
8. Al hacer clic en un bloque, se activa `editor/box` para edición inline

### Edición inline de bloque
1. Usuario hace clic en un bloque → `handler/block/edit-block!` se dispara
2. Se establece `editor/editing?` a true y `editor/block` al bloque target
3. `editor/box` se renderiza sobre el bloque:
   a. Muestra textarea con el contenido actual del bloque
   b. Posiciona el cursor en la ubicación del clic
   c. Registra listeners de teclado (Enter, Tab, Shift+Tab, Escape)
4. Durante la edición, el sistema de comandos slash está activo:
   a. Al escribir `/`, se despliega `editor/commands` con lista filtrada
   b. Comandos disponibles: `/TODO`, `/DONE`, `/NOW`, `/LATER`, `/DEADLINE`, `/SCHEDULED`, `/A`, `/B`, `/C`, `/query`, `/embed`, `/template`, etc.
   c. `filter-commands` filtra por contexto (page vs block)
5. Al pulsar Enter (sin comando activo):
   a. `handler/editor/save-current-block!` guarda el bloque editado
   b. `handler/editor/insert-new-block!` inserta un nuevo bloque debajo
6. Al pulsar Escape:
   a. Se descartan los cambios no guardados
   b. Se sale del modo de edición
7. Al pulsar Tab/Shift+Tab:
   a. Se indenta/outdenta el bloque vía `outliner/indent-outdent-blocks`

### Búsqueda inline de páginas y bloques
1. Usuario escribe `[[` en el editor → se abre `editor/page-search`
2. `search-pages` ejecuta búsqueda con debounce sobre DataScript:
   a. Busca páginas cuyo título contiene el query string (case-insensitive)
   b. Si `db-tag?` es true, busca también tags/clases
3. Resultados se muestran en dropdown con highlighting del match
4. `page-on-chosen-handler` gestiona la selección:
   a. Si la página existe → inserta referencia `[[page-name]]`
   b. Si no existe → `matched-pages-with-new-page` agrega opción "Create new page"
5. Para `[[` (block search), `editor/block-search` busca bloques por contenido
6. `node-render` renderiza cada resultado con highlighting del texto coincidente

### Drag & Drop de bloques
1. Usuario inicia arrastre de bloque → `*dragging?` se establece a true, `*dragging-block` al bloque origen
2. Durante el arrastre, al pasar sobre otro bloque → `*dragging-over-block` se actualiza
3. Se muestra indicador visual de posición de inserción (línea entre bloques)
4. Al soltar (drop) sobre un bloque destino:
   a. `*drag-to` se establece al bloque destino
   b. Se dispara `handler/block/move-blocks` con bloques origen y destino
   c. El outliner recalcula `block/order` del destino vía `db-order/gen-key`
   d. Se valida que no sea movimiento circular (no mover ancestro a descendiente)
5. Tras el drop, todos los átomos de drag se resetean a nil

### Sidebar izquierdo — navegación y favoritos
1. `left_sidebar/sidebar` se renderiza si `left-sidebar-open?` es true
2. Contiene:
   a. `graphs-selector` — selector de grafos (cambio entre repositorios)
   b. `sidebar-navigations` — accesos rápidos: Journals, All Pages, Graph View, Favorites, Recent
   c. `sidebar-favorites` — lista de páginas favoritas con toggle de favorito
   d. `sidebar-recent-pages` — últimas N páginas visitadas
3. En mobile breakpoint, el sidebar se cierra automáticamente si se navega a una página
4. `touching-x-offset` gestiona el gesto de swipe para abrir/cerrar en mobile

### Sidebar derecho — referencias y contexto
1. `right_sidebar/sidebar` se renderiza cuando `right-sidebar-open?` es true
2. `sidebar-item` representa cada bloque abierto en el sidebar:
   a. Muestra título del bloque o página
   b. Muestra contador de referencias/bloques hijos
   c. Permite cerrar items individualmente
3. `sidebar-inner` renderiza el contenido del item activo:
   a. Bloques hijos inmediatos
   b. Referencias entrantes
   c. Bloques vinculados
4. `sidebar-resizer` permite redimensionar entre 320px y 70% viewport
5. El estado de los items del sidebar se persiste entre sesiones

### Sistema de Queries
1. Usuario crea un bloque con query (ej: `{{query (and (task TODO) (priority A))}}`)
2. `query/custom-query` recibe `config` y `q` (query string o mapa):
   a. Determina si es DSL query (`dsl-query?`) o built-in query
   b. Si es DSL, parsea vía `frontend.db.query-dsl/query`
   c. Si es custom, ejecuta vía `frontend.db.query-custom/custom-query`
3. `custom-query-inner` ejecuta la query contra DataScript y obtiene resultados
4. `query-title` renderiza el título con:
   a. Texto de la query
   b. Conteo de resultados (`result-count`)
   c. Botón de colapso
5. Resultados se agrupan por página y se renderizan como bloques
6. El estado de colapso se persiste en `:block/properties` de DataScript
7. Queries reactivas se auto-refrescan cuando cambian los datos subyacentes

## Fluxos Alternativos
- **[Página no encontrada]:** Si `get-page-entity` retorna nil, se muestra "Page not found" con botón para crear la página con ese nombre 🟡
- **[Editor con formato distinto al de la página]:** Si el bloque tiene `:block/format` distinto al de su página, se usa el formato del bloque individual; si no, se hereda el de la página 🟢
- **[Asset no disponible (404)]:** Si un asset remoto falla al descargar, se muestra placeholder con icono de error y botón de reintento 🟡
- **[Query con cero resultados]:** Se muestra mensaje "No results" con el texto de la query y sugerencia de ajustar filtros 🟡
- **[Colapso/expansión de bloque con hijos]:** Al hacer clic en el bullet de un bloque con hijos, se alterna `:block/collapsed?` y la vista se actualiza reactivamente 🟢
- **[Comando slash en bloque vacío]:** Si el bloque está vacío y se escribe `/`, los comandos se filtran para mostrar solo los aplicables a un bloque nuevo 🟢
- **[Sidebar izquierdo en pantalla pequeña]:** Si el viewport es menor al breakpoint mobile, el sidebar se abre como overlay en lugar de empujar el contenido 🟡
- **[Drag & drop cancelado]:** Si el usuario suelta el bloque fuera de un destino válido, `*dragging?` se resetea y el bloque vuelve a su posición original 🟢
- **[Referencia a página con icono]:** Si una página tiene `icon::` property, la referencia `[[page]]` se renderiza con el icono junto al nombre 🟡

## Dependências
- `frontend.db` — acceso a entidades DataScript (`get-page`, `get-block-by-uuid`, `entity`, `pull`)
- `frontend.db.react` — queries reactivas que auto-refrescan componentes
- `frontend.db-mixins` — mixins Rum para queries DataScript declarativas
- `frontend.handler.editor` — manejo de eventos del editor (save, insert, delete, indent)
- `frontend.handler.block` — manejo de eventos de bloques (edit, select, drag, click)
- `frontend.handler.page` — manejo de eventos de páginas (create, delete, rename, favorite)
- `frontend.handler.ui` — manejo de eventos de UI (toggle sidebar, cambiar tema)
- `frontend.state` — átomos globales de estado (`sub`, `pub-event!`)
- `rum.core` — framework de componentes (fork de React)
- `datascript.core` — base de datos inmutable
- `logseq.db` — schema y funciones de base
- `logseq.outliner.tree` — operaciones de árbol para construir jerarquías de bloques

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Performance | Debounce de 1s en preview de página para evitar queries excesivas en hover rápido | `src/main/frontend/components/block.cljs:814-816` | 🟢 |
| Performance | Debounce en búsqueda inline de páginas para evitar búsquedas en cada keystroke | `src/main/frontend/components/editor.cljs` — `search-pages` con timer | 🟢 |
| Performance | Lazy loading de assets: imágenes remotas se descargan bajo demanda con tracking de progreso | `src/main/frontend/components/block.cljs:1012-1025` | 🟢 |
| Responsividad | Left sidebar se cierra automáticamente en mobile breakpoint para liberar espacio | `src/main/frontend/components/left_sidebar.cljs:420-423` | 🟢 |
| Responsividad | Right sidebar ancho mínimo 320px y máximo 70% viewport para evitar overflow | `src/main/frontend/components/right_sidebar.cljs:352-354` | 🟢 |
| UX | Comandos slash filtrados por contexto (page vs block) para evitar acciones inválidas | `src/main/frontend/components/editor.cljs` — `filter-commands` | 🟢 |
| Persistencia | Query collapse state se persiste en DataScript para sobrevivir a refrescos | `src/main/frontend/components/query.cljs:167` | 🟢 |
| Accesibilidad | Bloques reciclados son read-only y no permiten edición accidental | `src/main/frontend/components/block.cljs` — recycled block handling | 🟡 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

### Cenário: Renderizado de página con bloques y referencias
```gherkin
Dado una página "My Page" con 3 bloques top-level:
  - Bloque 1 (texto): "First bullet"
  - Bloque 2 (tarea TODO): "TODO Important task"
  - Bloque 3 (con hijos): "Parent" con hijo "Child item"
  Y la página tiene 2 referencias entrantes desde otras páginas
Cuando se navega a la ruta `:page` con `page-name` = "my page"
Então se renderiza `db-page-title` con el título "My Page"
  Y `page-blocks-cp` renderiza 3 bloques top-level
  Y el bloque 2 muestra el marcador TODO
  Y el bloque 3 muestra el bullet expandido con "Child item" indentado debajo
  Y `reference/references` muestra 2 referencias agrupadas
  Y ningún bloque está en modo edición
```

### Cenário: Edición inline y guardado con Enter
```gherkin
Dado un bloque existente con título "Old content"
  Y el usuario hace clic en el bloque
Cuando el editor `box` se activa sobre el bloque
  Y el usuario modifica el contenido a "New content"
  Y pulsa Enter
Então `handler/editor/save-current-block!` se dispara
  Y el bloque se persiste con `:block/title` = "New content" en DataScript
  Y un nuevo bloque vacío se inserta debajo del bloque editado
  Y el cursor se mueve al nuevo bloque en modo edición
```

### Cenário: Comando slash con filtrado por contexto
```gherkin
Dado un bloque en modo edición con formato :markdown
  Y el bloque pertenece a una página (no es página raíz)
Cuando el usuario escribe "/" en el editor
Então `commands` se despliega con la lista de comandos disponibles
  Y `filter-commands` filtra excluyendo comandos exclusivos de página (como "New Page")
  Y comandos como "/TODO", "/DONE", "/query" están disponibles
  Y al seleccionar "/TODO", se inserta "TODO " al inicio del bloque
```

### Cenário: Búsqueda de página con creación de nueva
```gherkin
Dado el editor activo con el usuario escribiendo "[[pro"
  Y existe una página "Projects" y otra "Process"
  Y no existe ninguna página llamada "Prologue"
Cuando `search-pages` busca "pro" en DataScript
Então se muestran "Projects" y "Process" en el dropdown
  Y `matched-pages-with-new-page` agrega "Create new page: pro" al final
  Y cada resultado muestra highlighting en "pro"
  Y al seleccionar "Create new page", se dispara `page/create` con título "pro"
```

### Cenário: Drag & drop entre bloques con recálculo de orden
```gherkin
Dado tres bloques siblings en orden: Bloque A, Bloque B, Bloque C
Cuando el usuario arrastra Bloque C y lo suelta entre A y B
Então `*dragging?` se activa con Bloque C
  Y `*drag-to` se establece a la posición entre A y B
  Y `move-blocks` se dispara con target = Bloque A (como sibling, above B)
  Y `block/order` de Bloque C se recalcula vía `db-order/gen-key` entre los órdenes de A y B
  Y el nuevo orden visual es: Bloque A, Bloque C, Bloque B
```

### Cenário: Sidebar derecho con ancho restringido
```gherkin
Dado el right sidebar abierto con ancho actual de 400px
Cuando el usuario arrastra el `sidebar-resizer` hasta 800px
Então el sidebar no excede 70% del viewport total
  Y si el viewport es 1000px, el máximo es 700px
  Y el contenido del sidebar se re-renderiza con el nuevo ancho
  Y el ancho mínimo nunca es menor a 320px aunque el usuario intente reducir más
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| Renderizado de bloques en outliner | Must | Componente central de la aplicación — toda página lo requiere |
| Editor inline con guardado | Must | Sin edición la aplicación no es funcional |
| Renderizado de página con bloques | Must | Vista principal de contenido — sin esto no hay app |
| Comandos slash con filtrado | Must | Interfaz principal para formateo y acciones rápidas |
| Drag & drop de bloques | Must | Reorganización de contenido esencial para PKM |
| Sidebar izquierdo con navegación | Must | Navegación principal entre grafos y páginas |
| Sistema de queries con resultados | Should | Importante pero la app funciona sin queries visuales |
| Renderizado de assets (imágenes, PDFs) | Should | Mejora significativa de UX pero no bloqueante |
| Sidebar derecho con referencias | Should | Navegación contextual valiosa pero no esencial |
| Preview de página en hover | Should | Mejora de descubribilidad pero con alternativa (clic) |
| Breadcrumbs de navegación | Could | Conveniencia de UI, raramente usada |
| Vista de grafo global | Could | Visualización complementaria, no necesaria para edición |
| Tema/Theme switcher | Could | Preferencia visual del usuario |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Cenários de Borda

### 1. Bloque con contenido extremadamente largo
**Situação:** Un bloque contiene 50,000+ caracteres de texto (ej: log dump, datos copiados).
**Comportamento esperado:**
- El editor limita el renderizado inicial a un número configurable de líneas visibles
- Se muestra indicador "Show more..." al final del contenido truncado
- La edición del bloque completo es posible pero con advertencia de performance
- El guardado se procesa en worker thread para no bloquear la UI
- La búsqueda full-text indexa el contenido completo sin truncar

### 2. Página con más de 1000 bloques top-level
**Situação:** Una página contiene 1000+ bloques directos sin colapsar.
**Comportamento esperado:**
- `page-blocks-cp` implementa windowed rendering: solo se renderizan los bloques visibles en viewport
- Los bloques fuera del viewport se renderizan como placeholders vacíos con altura estimada
- Al hacer scroll, los placeholders se reemplazan por bloques reales (virtual scrolling)
- La query inicial de DataScript carga todos los bloques pero el renderizado es lazy
- Los bloques colapsados no renderizan sus hijos hasta expandirse

### 3. Colapso masivo de bloques anidados
**Situação:** Usuario colapsa un bloque que tiene 500+ descendientes en múltiples niveles de anidación.
**Comportamento esperado:**
- `block/collapsed?` se establece a true solo en el bloque padre
- Los hijos no se eliminan de DataScript, solo se ocultan en la vista
- El indicador de colapso muestra el conteo total de bloques ocultos (ej: "(500+)")
- Al expandir, los bloques se renderizan lazy con windowed rendering
- La operación de colapso/expansión es O(1) en el padre, no recorre todos los hijos

### 4. Cambio rápido entre páginas con queries reactivas activas
**Situação:** Usuario navega rápidamente entre 5 páginas en menos de 2 segundos, cada una con múltiples queries reactivas.
**Comportamento esperado:**
- Queries de la página anterior se des-suscriben automáticamente al desmontar el componente
- `rum.core` gestiona el ciclo de vida `:will-unmount` para limpiar suscripciones
- Las queries de la nueva página se inician inmediatamente
- Si una query tarda más de 300ms en resolverse, se muestra skeleton loader
- No hay memory leak por queries huérfanas de páginas anteriores

### 5. Editor con composición IME (Input Method Editor)
**Situação:** Usuario con teclado japonés, chino o coreano usa IME para escribir caracteres.
**Comportamento esperado:**
- `editor/in-composition?` se establece a true durante la composición IME
- Eventos de teclado (Enter, Escape) se suprimen durante composición activa
- El guardado del bloque solo ocurre tras `compositionend`, no en cada keystroke
- El contenido en composición se muestra correctamente en el textarea
- Al finalizar composición, se dispara `save-current-block!` normalmente

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `src/main/frontend/components/container.cljs` | `root-container` | 🟢 |
| `src/main/frontend/components/container.cljs` | `main` | 🟢 |
| `src/main/frontend/components/container.cljs` | `custom-context-menu` | 🟢 |
| `src/main/frontend/components/container.cljs` | `app-context-menu-observer` | 🟡 |
| `src/main/frontend/components/editor.cljs` | `box` | 🟢 |
| `src/main/frontend/components/editor.cljs` | `commands` | 🟢 |
| `src/main/frontend/components/editor.cljs` | `filter-commands` | 🟢 |
| `src/main/frontend/components/editor.cljs` | `search-pages` | 🟢 |
| `src/main/frontend/components/editor.cljs` | `page-search` | 🟢 |
| `src/main/frontend/components/editor.cljs` | `block-search` | 🟢 |
| `src/main/frontend/components/editor.cljs` | `node-render` | 🟢 |
| `src/main/frontend/components/block.cljs` | `block` | 🟢 |
| `src/main/frontend/components/block.cljs` | `page-cp` | 🟢 |
| `src/main/frontend/components/block.cljs` | `page-reference` | 🟢 |
| `src/main/frontend/components/block.cljs` | `asset-container` | 🟢 |
| `src/main/frontend/components/block.cljs` | `resizable-image` | 🟢 |
| `src/main/frontend/components/block.cljs` | `timestamp` | 🟢 |
| `src/main/frontend/components/block.cljs` | `breadcrumb` | 🟢 |
| `src/main/frontend/components/block.cljs` | `open-pdf-file` | 🟡 |
| `src/main/frontend/components/page.cljs` | `page-cp` | 🟢 |
| `src/main/frontend/components/page.cljs` | `page-blocks-cp` | 🟢 |
| `src/main/frontend/components/page.cljs` | `db-page-title` | 🟢 |
| `src/main/frontend/components/page.cljs` | `global-graph` | 🟢 |
| `src/main/frontend/components/page.cljs` | `get-page-name` | 🟢 |
| `src/main/frontend/components/page.cljs` | `get-page-entity` | 🟢 |
| `src/main/frontend/components/query.cljs` | `custom-query` | 🟢 |
| `src/main/frontend/components/query.cljs` | `custom-query-inner` | 🟢 |
| `src/main/frontend/components/query.cljs` | `query-title` | 🟢 |
| `src/main/frontend/components/left_sidebar.cljs` | `sidebar` | 🟢 |
| `src/main/frontend/components/left_sidebar.cljs` | `sidebar-container` | 🟢 |
| `src/main/frontend/components/left_sidebar.cljs` | `sidebar-navigations` | 🟢 |
| `src/main/frontend/components/left_sidebar.cljs` | `sidebar-favorites` | 🟢 |
| `src/main/frontend/components/left_sidebar.cljs` | `sidebar-recent-pages` | 🟢 |
| `src/main/frontend/components/right_sidebar.cljs` | `sidebar` | 🟢 |
| `src/main/frontend/components/right_sidebar.cljs` | `sidebar-inner` | 🟢 |
| `src/main/frontend/components/right_sidebar.cljs` | `sidebar-item` | 🟢 |
| `src/main/frontend/components/right_sidebar.cljs` | `sidebar-resizer` | 🟢 |
| `src/main/frontend/components/property.cljs` | `property` | 🟡 |
| `src/main/frontend/components/property/dialog.cljs` | `property-dialog` | 🟡 |
| `src/main/frontend/components/property/value.cljs` | `property-value` | 🟡 |
| `src/main/frontend/components/property/config.cljs` | `property-config` | 🟡 |
| `src/main/frontend/components/reference.cljs` | `references` / `unlinked-references` | 🟡 |
| `src/main/frontend/components/recycle.cljs` | Recycle bin view | 🟡 |
| `src/main/frontend/components/content.cljs` | Content rendering | 🟡 |
| `src/main/frontend/components/header.cljs` | `header` | 🟡 |
| `src/main/frontend/components/settings.cljs` | Settings panel | 🟡 |
| `src/main/frontend/components/datepicker.cljs` | Date picker | 🟡 |
| `src/main/frontend/components/theme.cljs` | Theme system | 🟡 |
| `src/main/frontend/components/svg.cljs` | SVG utilities | 🟡 |
