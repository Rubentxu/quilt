# Manual de Usuario — Quilt

> **Quilt** es tu espacio de trabajo inteligente basado en bloques. Combina un outliner estructurado,
> un grafo de conocimiento y agentes de IA en un único workspace local-first.

---

## Primeros Pasos

### 1. Iniciar Quilt

```bash
just dev
```

Esto levanta:
- **Backend API** en `http://localhost:3737`
- **Frontend React** en `http://localhost:5173`
- **WASM** para operaciones de grafo (si tu plataforma lo soporta)

### 2. Configurar la API Key

En la primera ejecución, Quilt genera una API key y la imprime en la consola del servidor:

```
✅ API key generated: 4eb04d25-ba93-44e3-baec-9c21ae5a9a2c
```

Copia esa key y créala en `quilt-ui/.env`:

```env
VITE_QUILT_API_KEY=4eb04d25-ba93-44e3-baec-9c21ae5a9a2c
```

Abre `http://localhost:5173` en tu navegador.

---

## Navegación

### Redirección al diario de hoy

Al abrir `/` (la raíz), Quilt te redirige automáticamente al diario de hoy:

```
/ → /journal/2026-06-07
```

![Home redirect — diario de hoy](screenshots/01-home-redirect.png)

### Vista de Diarios

Los diarios son páginas diarias generadas automáticamente. Navega con los botones
**Previous day** y **Next day**, o haz clic en **Today** para volver al día actual.

![Journal page](screenshots/04-journal-page.png)

### Lista de Páginas

Todas las páginas del workspace. Crea nuevas desde el journal o con **Cmd+Shift+K → New page**.

![Lista de páginas](screenshots/05-pages-list.png)

---

## Paleta de Comandos — `Cmd+Shift+K`

El centro de comandos de Quilt. Pulsa **`Cmd+Shift+K`** (Mac) o **`Ctrl+Shift+K`** (Linux/Windows)
desde cualquier lugar de la aplicación para abrir la paleta.

![Paleta de comandos](screenshots/02-command-palette.png)

### Qué puedes hacer desde la paleta

| Acción | Descripción |
|--------|-------------|
| **Nueva página** | Crear una página en blanco |
| **Buscar** | Buscar en todo el workspace |
| **Cambiar tema** | Alternar entre modo claro y oscuro |
| **Layout: Default / Focus / Review** | Cambiar preset de paneles |
| **Toggle Sidebar** | Mostrar/ocultar el sidebar |
| **Toggle Backlinks** | Mostrar/ocultar el panel de referencias |

### Panel de Layout (presets de paneles)

Accede desde el botón **Layout** en la barra superior o desde `Cmd+Shift+K`:

![Dashboard layout — presets de paneles](screenshots/08-dashboard-layout.png)

| Preset | Paneles visibles |
|--------|-----------------|
| **Default** | Sidebar + Journal + Backlinks |
| **Focus** | Solo el journal, sin distracciones |
| **Review** | Journal + Backlinks expandidos |

También puedes togglear paneles individualmente (Sidebar, Backlinks, Agent Activity).

---

## Slash Commands — `/`

Los slash commands transforman bloques en diferentes tipos semánticos.
En cualquier bloque, escribe `/` para abrir el menú de comandos.

![Slash command menu](screenshots/06-slash-command-menu.png)

### Comandos disponibles

| Comando | Acción |
|---------|--------|
| `/h1`, `/h2`, `/h3` | Convertir a encabezado (establece `level:: 1/2/3`) |
| `/code` | Bloque de código |
| `/quote` | Cita |
| `/task` | **Rol de tarea** — establece `type:: task` + `status:: todo` |
| `/query` | **Rol de query** — establece `type:: query` + `dsl::` (pide el DSL por prompt) |
| `/card` | **Forma de tarjeta** — establece `card-shape::` (prompt para elegir forma) |
| `/divider` | Separador visual |
| `/bullet`, `/numbered` | Lista de puntos o numerada |

### Roles de bloque

Quilt usa **propiedades** para definir roles semánticos (no solo visuales).
Un bloque con `type:: task` es una tarea; con `type:: query` es una consulta;
con `type:: agent-run` es un bloque de ejecución de agente.

#### `/task` — Rol de tarea

Establece `type:: task` y `status:: todo`. Después puedes cambiar el estado
a `done`, `cancelled`, etc.

#### `/query` — Rol de query

Establece `type:: query` y pide un DSL (Domain Specific Language) para definir
la consulta. El query puede ser ejecutado por el motor de búsqueda de Quilt.

#### `/card` — Forma de tarjeta

Establece `card-shape::` con los valores: `content`, `reference`, `presentation`,
`article`, `note`.

---

## Grafo de Conocimiento

Visualiza las conexiones entre tus bloques. El grafo es interactivo:
clic en un nodo abre la página, scroll para zoom, drag para mover.

> **Tema oscuro**: el grafo detecta `data-theme="dark"` y se renderiza
> con un fondo oscuro automático.

![Graph view — modo oscuro](screenshots/03-graph-dark-mode.png)

---

## Búsqueda

Busca en todo el workspace desde el input del sidebar o con `Cmd+Shift+K → Buscar`.

Incluye autocompletado de páginas, resultados de búsqueda full-text, y filtros
por propiedad.

![Search modal](screenshots/10-search-modal.png)

---

## Vistas guardadas (`type:: view`)

Una **vista guardada** es un bloque que referencia un query existente y define
cómo renderizarlo. Modelo:

```
Bloque query:
  type:: query
  dsl:: (and (task todo) (project "mi-proyecto"))

Bloque vista:
  type:: view
  view-type:: kanban
  data-source:: <uuid-del-bloque-query>
  view-name:: Mis tareas
  group-by:: priority
```

Múltiples vistas pueden compartir el mismo query con diferentes renderizadores:
`kanban`, `table`, `list`, `graph`, `cards`, `calendar`, `timeline`.

---

## Bloques de Agente (`type:: agent-run`)

Cuando un agente externo ejecuta operaciones en Quilt, cada ejecución se registra
como un bloque con `type:: agent-run`. El bloque muestra:

- Nombre del agente (`agent::`)
- Modelo usado (`model::`)
- Estado de ejecución: `Queued` → `Running` → `Completed` / `Failed` / `Cancelled`
- Timestamp de inicio (`started-at::`)

Estados con colores:

| Estado | Color |
|--------|-------|
| Queued / Cancelled | Gris |
| Running | Azul |
| Completed | Verde |
| Failed | Rojo |

---

## Fechas en Lenguaje Natural

En valores de propiedad puedes escribir fechas en lenguaje natural:

| Escrito | Se resuelve a |
|---------|--------------|
| `today` | La fecha actual |
| `tomorrow` | La fecha de mañana |
| `yesterday` | La fecha de ayer |

Ejemplo: una propiedad `due:: today` en una tarea se resuelve automáticamente
al día actual.

---

## Block Zoom — `?zoom=$blockId`

Zoom a cualquier bloque haciendo su contenido el foco de la vista.
Usa la URL con el parámetro `?zoom=` para deep-link a un bloque específico:

```
/journal/2026-06-07?zoom=abc123
```

Esto abre el journal centrado en el bloque `abc123`, con su contenido expandido
y el resto de la página difuminado.

---

## Configuración

Accede desde el menú superior → **Settings**.

![Settings page](screenshots/09-settings-page.png)

Aquí puedes:
- Cambiar el tema (claro/oscuro)
- Configurar el polling interval para sincronización
- Gestionar la API key

---

## Atajos de Teclado

| Atajo | Acción |
|-------|--------|
| `Cmd+Shift+K` / `Ctrl+Shift+K` | Abrir paleta de comandos |
| `Cmd+Z` / `Ctrl+Z` | Deshacer última acción |
| `Enter` en un bloque | Crear bloque hermano |
| `Tab` | Indentar bloque (hijo) |
| `Shift+Tab` | Des-indentar bloque (padre) |
| `/` al inicio de un bloque | Abrir menú de slash commands |
| `Esc` | Cerrar menús/modales |

---

## Arquitectura de Propiedades

En Quilt todo es un **bloque** con **propiedades tipadas**. No hay campos de frontmatter
como en otros sistemas — las propiedades se almacenan en una columna `properties` (JSONB en SQLite).

### Propiedades reservadas

| Propiedad | Tipo | Uso |
|-----------|------|-----|
| `type::` | role | Rol del bloque: `task`, `query`, `view`, `agent-run`, `comment`... |
| `status::` | select | Estado: `todo`, `done`, `running`, `cancelled`... |
| `priority::` | select | Prioridad: `A`, `B`, `C` |
| `due::` | date | Fecha de vencimiento (soporta NL: today/tomorrow/yesterday) |
| `dsl::` | string | DSL de consulta (para bloques `type:: query`) |
| `data-source::` | block-ref | UUID del bloque fuente (para vistas) |
| `view-type::` | select | Tipo de renderer: `kanban`, `table`, `list`... |
| `agent::` | string | Nombre del agente |
| `model::` | string | Modelo usado por el agente |
| `run-status::` | select | Estado de ejecución del agente |
| `card-shape::` | select | Forma de tarjeta visual |
| `level::` | number | Nivel de encabezado (1-3) |

---

*Generado automáticamente — `just dev` + Playwright CLI*
