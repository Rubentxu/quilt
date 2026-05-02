# User Stories — Navegación

> **Proyecto**: Logseq
> **Generado por**: reversa-writer
> **Fecha**: 2026-05-02
> **Nivel**: detalhado
> **Escala**: 🟢 CONFIRMADO | 🟡 INFERIDO | 🔴 LACUNA

---

## Flujo 1: Abrir Página por Nombre

### US-NAV-01: Abrir página existente escribiendo [[nombre]] 🟢 CONFIRMADO

**Contexto**: El usuario navega a una página existente usando la sintaxis de wiki-link `[[nombre-pagina]]`.

**Rastreabilidad**: `frontend/components/editor.cljs` → `search-pages`, `page-on-chosen-handler`, `matched-pages-with-new-page`

```gherkin
Dado que existe la página "Arquitectura del Sistema" en el grafo
  Y el usuario está editando un bloque
Cuando el usuario escribe "[[Arq"
Então se muestra un dropdown de autocompletado con páginas que coinciden
  Y "Arquitectura del Sistema" aparece en las opciones
  Y la búsqueda es case-insensitive
  Y al seleccionar la página, se inserta `[[Arquitectura del Sistema]]` como enlace
  Y al hacer clic en el enlace, se navega a la página "Arquitectura del Sistema"
```

---

### US-NAV-02: Abrir página que no existe (creación on-demand) 🟢 CONFIRMADO

**Contexto**: El usuario navega a una página que aún no ha sido creada; el sistema ofrece crearla.

**Rastreabilidad**: `frontend/components/editor.cljs` → `matched-pages-with-new-page`, `frontend/handler/page.cljs` → `:page/create`

```gherkin
Dado que NO existe la página "Nueva Investigación" en el grafo
  Y el usuario escribe "[[Nueva Inv"
Cuando el usuario ve el dropdown de autocompletado
Então se muestra la opción "Create 'Nueva Investigación'" además de los matches existentes
  Y al seleccionar "Create", se dispara el evento `:page/create`
  Y se crea una nueva página con nombre "nueva investigación" (lowercase)
  Y la página recibe un UUID generado automáticamente
  Y se navega a la nueva página (vacía, lista para recibir contenido)
```

---

### US-NAV-03: Abrir página desde la barra lateral izquierda (recent pages) 🟡 INFERIDO

**Contexto**: El usuario accede a páginas recientes o favoritas desde el panel de navegación izquierdo.

**Rastreabilidad**: `frontend/components/left_sidebar.cljs`

```gherkin
Dado que el usuario ha visitado recientemente las páginas:
  - "Daily Standup"
  - "Proyecto Alpha"
  - "Notas de Diseño"
Cuando el usuario abre la barra lateral izquierda
Então se muestra una lista de páginas recientes ordenadas por última visita
  Y "Daily Standup" aparece como la más reciente
  Y al hacer clic en "Proyecto Alpha", se navega directamente a esa página
  Y la página abierta se mueve al tope de la lista de recientes
```

---

### US-NAV-04: Abrir página usando el comando de búsqueda rápida (Ctrl+K / Cmd+K) 🟡 INFERIDO

**Contexto**: El usuario usa un atajo de teclado para abrir el diálogo de búsqueda de páginas.

**Rastreabilidad**: `frontend/components/editor.cljs` → `search-pages`, `frontend/commands.cljs`

```gherkin
Dado que el usuario está en cualquier vista del grafo
Cuando el usuario presiona Ctrl+K (o Cmd+K en macOS)
Então se abre un diálogo modal de búsqueda rápida de páginas
  Y el foco se coloca en el campo de búsqueda
  Y al escribir "reunión", se filtran las páginas en tiempo real
  Y al seleccionar una página y presionar Enter, se navega a ella
  Y el diálogo se cierra automáticamente tras la navegación
```

---

### US-NAV-05: Navegar a una página a través de namespace jerárquico 🟢 CONFIRMADO

**Contexto**: El usuario navega entre páginas organizadas con namespaces (ej: `proyecto/alfa/notas`).

**Rastreabilidad**: `deps/graph-parser/src/logseq/graph_parser/extract.cljc` → `build-pages-aux`, namespace resolution

```gherkin
Dado que existen las páginas:
  - "proyecto" (namespace padre)
  - "proyecto/alfa" (página hija con namespace)
  - "proyecto/alfa/notas" (página nieta)
Cuando el usuario navega a "proyecto/alfa/notas"
Então la página se abre con el namespace completo resuelto
  Y en la UI se muestra el breadcrumb: proyecto > alfa > notas
  Y las referencias `[[proyecto/alfa/notas]]` se resuelven correctamente
  Y la página hereda el formato del namespace padre si está configurado
```

---

## Flujo 2: Journal (Páginas Diarias)

### US-NAV-06: Abrir el journal del día actual 🟢 CONFIRMADO

**Contexto**: El usuario accede a la página de journal correspondiente al día de hoy para tomar notas diarias.

**Rastreabilidad**: `frontend/db/model.cljs` → `get-today-journal-page`, `frontend/components/journal.cljs`

```gherkin
Dado que hoy es 2026-05-02
  Y el usuario hace clic en "Journal" o presiona el atajo correspondiente
Cuando se solicita el journal del día actual
Então el sistema busca una página con `:block/journal-day` = 20260502
  Y si la página YA existe, se abre con todo su contenido previo
  Y si la página NO existe, se crea automáticamente:
    - Con nombre "May 2nd, 2026" (o formato configurado)
    - Con `:block/journal-day` = 20260502
    - Con tag `:logseq.class/Journal`
    - Con el formato de fecha configurado por el usuario
  Y la página se abre en la vista principal
```

---

### US-NAV-07: Navegar a journals anteriores y siguientes (calendario) 🟡 INFERIDO

**Contexto**: El usuario navega entre días consecutivos del journal usando controles de calendario.

**Rastreabilidad**: `frontend/db/model.cljs` → `get-latest-journals`, `get-journal-page`

```gherkin
Dado que el usuario está en el journal del 2026-05-02
  Y existen journals para los días 2026-04-30, 2026-05-01, 2026-05-02
Cuando el usuario hace clic en "día anterior" o usa el atajo de teclado
Então se navega al journal del 2026-05-01
  Y si el journal del 2026-05-01 existe, se abre con su contenido
  Y si no existe, se crea automáticamente al navegar
  Y el usuario puede seguir navegando hacia atrás hasta 2026-04-30
  Y el calendario visual resalta los días que tienen contenido
```

---

### US-NAV-08: Acceder al journal desde una referencia de fecha en un bloque 🟢 CONFIRMADO

**Contexto**: Un bloque contiene una fecha referenciada y el usuario puede navegar al journal de esa fecha.

**Rastreabilidad**: `frontend/components/block.cljs` → renderizado de fechas, `frontend/db/model.cljs` → `get-journal-page`

```gherkin
Dado que un bloque contiene el texto:
  "Reunión programada para [[2026-05-15]]"
  Y la fecha se renderiza como un enlace cliqueable
Cuando el usuario hace clic en "2026-05-15"
Então se navega al journal correspondiente al 15 de mayo de 2026
  Y si el journal ya existe, se abre con su contenido
  Y si no existe, se crea automáticamente con `:block/journal-day` = 20260515
  Y se preserva el contexto de navegación (el usuario puede volver atrás)
```

---

### US-NAV-09: Ver lista de journals recientes (últimos N días) 🟢 CONFIRMADO

**Contexto**: El usuario consulta los journals más recientes para tener una visión general de su actividad.

**Rastreabilidad**: `frontend/db/model.cljs` → `get-latest-journals`

```gherkin
Dado que el usuario ha creado journals en los últimos 7 días
Cuando el sistema carga la lista de journals recientes (n = 7)
Então se retornan las 7 páginas de journal más recientes
  Y se ordenan por `:block/journal-day` descendente (más reciente primero)
  Y cada entrada muestra la fecha y un preview del contenido
  Y los días sin journal no aparecen en la lista
```

---

### US-NAV-10: El journal tiene atributos protegidos contra modificaciones accidentales 🟢 CONFIRMADO

**Contexto**: El sistema protege atributos críticos del journal para mantener la integridad estructural.

**Rastreabilidad**: `src/test/frontend/worker/pipeline_test.cljs` → `journal-protected-update-attrs`

```gherkin
Dado que existe un journal con `:block/journal-day` = 20260502
  Y `:block/name` = "May 2nd, 2026"
Cuando un plugin o proceso intenta modificar `:block/journal-day` o `:block/name`
Então la operación es rechazada
  Y se lanza excepción con tipo `:journal-page-protected-attr-updated`
  Y el atributo protegido mantiene su valor original
  Y otros atributos del journal (contenido, propiedades) sí pueden modificarse normalmente
```

---

## Flujo 3: Referencias y Backlinks

### US-NAV-11: Ver backlinks de una página (páginas que la referencian) 🟢 CONFIRMADO

**Contexto**: El usuario consulta todas las páginas y bloques que enlazan a la página actual.

**Rastreabilidad**: `frontend/components/page.cljs` → sección de backlinks, `frontend/components/reference.cljs` → renderizado de referencias

```gherkin
Dado que la página "Proyecto Alpha" es referenciada desde:
  - "Reuniones" → bloque "Ver [[Proyecto Alpha]]"
  - "Notas" → bloque "Relacionado con [[Proyecto Alpha]]"
  - "Tareas" → bloque "Completar diseño de [[Proyecto Alpha]]"
Cuando el usuario abre la página "Proyecto Alpha" y consulta los backlinks
Então se muestran 3 referencias entrantes (backlinks)
  Y cada backlink muestra:
    - El nombre de la página de origen ("Reuniones", "Notas", "Tareas")
    - El contenido del bloque que contiene la referencia
    - Un enlace para navegar directamente al bloque origen
  Y los backlinks se agrupan por página de origen
  Y los backlinks se actualizan en tiempo real cuando se añaden nuevas referencias
```

---

### US-NAV-12: Ver referencias salientes de una página (linked references) 🟢 CONFIRMADO

**Contexto**: El usuario consulta todas las páginas que son referenciadas desde la página actual.

**Rastreabilidad**: `frontend/components/page.cljs`, `frontend/db/model.cljs` → `get-block-by-uuid`, queries de `:block/refs`

```gherkin
Dado que la página "Proyecto Alpha" contiene bloques que referencian a:
  - [[Tecnología Stack]]
  - [[Equipo de Desarrollo]]
  - [[Presupuesto Q2]]
Cuando el usuario abre "Proyecto Alpha" y consulta las referencias salientes
Então se muestran las 3 páginas referenciadas como linked references
  Y cada referencia muestra:
    - El nombre de la página destino
    - Cuántos bloques en esta página la referencian
    - Un enlace directo para navegar a la página destino
  Y las referencias se agrupan por página destino
```

---

### US-NAV-13: Navegar a un bloque específico usando block reference ((uuid)) 🟢 CONFIRMADO

**Contexto**: El usuario hace clic en una referencia de bloque `((uuid))` para navegar directamente a ese bloque.

**Rastreabilidad**: `frontend/components/block.cljs` → renderizado de block refs, `frontend/db/model.cljs` → `get-block-by-uuid`

```gherkin
Dado que existe un bloque con UUID "550e8400-e29b-41d4-a716-446655440000"
  Y ese bloque contiene el texto "Decisión importante de arquitectura"
  Y otro bloque en diferente página contiene `((550e8400-e29b-41d4-a716-446655440000))`
Cuando el usuario hace clic en la referencia `((550e8400-...))`
Então se navega directamente al bloque "Decisión importante de arquitectura"
  Y el bloque se resalta/scroll en la vista
  Y se muestra el contexto del bloque (página y bloques circundantes)
  Y se preserva el historial de navegación para volver atrás
```

---

### US-NAV-14: Ver bloques embebidos (embed references) en contexto 🟡 INFERIDO

**Contexto**: El usuario usa `{{embed ((uuid))}}` para incrustar el contenido de un bloque en otra página.

**Rastreabilidad**: `frontend/components/block.cljs` → renderizado de embeds

```gherkin
Dado que existe un bloque "Checklist de deploy" con UUID "abc-123"
  Y el usuario escribe `{{embed ((abc-123))}}` en otra página
Cuando la página se renderiza
Então el contenido del bloque "Checklist de deploy" se muestra inline en la página actual
  Y el bloque se renderiza con su formato original (markdown, propiedades, etc.)
  Y los cambios al bloque original se reflejan automáticamente en el embed
  Y el embed es de solo lectura en el contexto de la página actual
  Y al hacer clic en el embed, se puede navegar al bloque original
```

---

### US-NAV-15: Ver referencias entre bloques con breadcrumb de navegación 🟢 CONFIRMADO

**Contexto**: El usuario ve un breadcrumb que muestra la jerarquía de navegación al consultar referencias.

**Rastreabilidad**: `frontend/components/block.cljs` → `breadcrumb`

```gherkin
Dado que la página "Proyecto Alpha > Diseño > UI" tiene un backlink desde "Reuniones"
Cuando el usuario consulta los backlinks de "UI"
Então cada backlink muestra el breadcrumb completo de origen
  Y para el backlink desde "Reuniones", se muestra: Reuniones > Bloque X
  Y el breadcrumb es cliqueable en cada nivel
  Y al hacer clic en "Reuniones", se navega a esa página
  Y al hacer clic en "Bloque X", se navega directamente al bloque específico
```

---

### US-NAV-16: Ver backlinks jerárquicos agrupados (block references hierarchy) 🟡 INFERIDO

**Contexto**: Los backlinks se organizan jerárquicamente cuando provienen de bloques indentados.

**Rastreabilidad**: `frontend/components/reference.cljs`, `deps/outliner/src/logseq/outliner/tree.cljs` → `blocks->vec-tree`

```gherkin
Dado que la página "Concepto Clave" es referenciada desde:
  - Bloque A (nivel 1) → "Ver [[Concepto Clave]]"
  - Bloque B (nivel 2, hijo de A) → "Ampliar [[Concepto Clave]]"
  - Bloque C (nivel 1) → "Resumen de [[Concepto Clave]]"
Cuando el usuario consulta los backlinks de "Concepto Clave"
Então los backlinks se muestran respetando la jerarquía
  Y el Bloque A aparece al nivel 1
  Y el Bloque B aparece indentado bajo el Bloque A
  Y el Bloque C aparece al nivel 1 (separado del grupo A-B)
  Y el contexto jerárquico ayuda a entender la relación entre referencias
```

---

## Cenários de Borda

### NAV-BORDE-1: Navegación a página con nombre que contiene caracteres Unicode 🟢 CONFIRMADO

**Contexto**: El nombre de página contiene caracteres no-ASCII (chino, japonés, árabe, emojis, etc.).

**Comportamiento esperado**:
- Los caracteres Unicode se preservan sin modificaciones en el nombre de página
- La búsqueda y navegación funcionan con nombres como "日本語ノート", "한국어", "المعرفة"
- Los journals con formatos de fecha localizados se manejan correctamente
- La normalización lowercase aplica solo a caracteres ASCII

---

### NAV-BORDE-2: Navegación con página que tiene múltiples alias 🟡 INFERIDO

**Contexto**: Una página tiene definidos varios alias (`alias:: nombre1, nombre2`) y es referenciada por cualquiera de ellos.

**Comportamiento esperado**:
- Referencias `[[alias1]]` y `[[alias2]]` navegan a la misma página
- Los backlinks se consolidan bajo el nombre canónico de la página
- En el autocompletado, se muestran tanto el nombre canónico como los alias
- Al guardar, los alias se resuelven al nombre canónico

---

### NAV-BORDE-3: Referencias circulares entre páginas (A ↔ B) 🟡 INFERIDO

**Contexto**: Dos páginas se referencian mutuamente creando un ciclo.

**Comportamiento esperado**:
- Ambas páginas muestran backlinks de la otra sin problemas
- No hay loop infinito en la renderización porque la UI solo muestra un nivel de backlinks
- Las queries DSL que navegan referencias pueden necesitar límites de profundidad
- El grafo de conocimiento visual puede mostrar la conexión bidireccional

---

### NAV-BORDE-4: Journal de una fecha en año bisiesto (29 de febrero) 🟡 INFERIDO

**Contexto**: El sistema debe manejar correctamente fechas especiales como 29 de febrero.

**Comportamiento esperado**:
- `2028-02-29` se reconoce como fecha válida en año bisiesto
- `:block/journal-day` = 20280229
- El journal se crea y se referencia normalmente
- `2027-02-29` (año no bisiesto) no debería generarse; el sistema podría rechazarlo o normalizarlo

---

## Resumen de cobertura

| Flujo | Escenarios | Confianza predominante |
|-------|-----------|----------------------|
| Abrir Página por Nombre | 5 | 🟢 CONFIRMADO (3), 🟡 INFERIDO (2) |
| Journal (Páginas Diarias) | 5 | 🟢 CONFIRMADO (4), 🟡 INFERIDO (1) |
| Referencias y Backlinks | 6 | 🟢 CONFIRMADO (4), 🟡 INFERIDO (2) |
| Cenários de Borda | 4 | 🟢 CONFIRMADO (1), 🟡 INFERIDO (3) |

---

*Documento generado automáticamente por Reversa Writer*
