# Quilt — AI-first Knowledge Graph

PKM (Personal Knowledge Management) con UI estilo Logseq que expone un MCP server para que agentes AI externos colaboren con el usuario sobre el grafo de conocimiento. Quilt no tiene IA interna.

## Language

### Grafo
El conocimiento del usuario almacenado en SQLite: páginas, bloques jerárquicos, referencias, propiedades y tags. Un solo grafo por usuario.
_Avoid_: base de datos, knowledge base, vault

### Bloque
Unidad atómica de contenido en el outliner. Tiene UUID inmutable, contenido markdown, propiedades tipadas, refs a otros bloques/páginas, y posición jerárquica (parent, order, level).
_Avoid_: nodo, item, entrada

### Página
Contenedor de bloques. Puede ser normal, journal, o namespace. Identificada por nombre (lowercase, único).
_Avoid_: documento, archivo, nota

### Journal
Página diaria identificada por `journal_day` (YYYYMMDD). Se crea automáticamente al navegar. Propiedad `journal:: true`.
_Avoid_: daily note, diario

### Outliner
El modelo de interacción principal: bloques jerárquicos con indentación, colapso, reordenamiento y edición inline. Inspirado en Logseq.
_Avoid_: editor, tree view

### MCP Server
La interfaz pública de Quilt para agentes AI externos. Expone tools, resources y notifications via Model Context Protocol. Los nombres de tools usan prefijo `quilt_`.
_Avoid_: API, backend, servicio

### Agente
Un agente AI externo (Claude, GPT, etc.) que interactúa con el grafo via MCP. No es parte de Quilt. El usuario lo dirige desde fuera (ej: Claude Code).
_Avoid_: bot, asistente, IA interna

### Template
Bloque con propiedad `template:: nombre` que define estructura (headings, propiedades, slots) y actúa como contrato entre agente y usuario. El agente rellena slots via MCP.
_Avoid_: plantilla, formato

### Template Page
Página con nombre prefijado por `template/` que define la estructura, propiedades y comportamiento visual de un Template. Es el origen de la activación — un Bloque con `template:: <nombre>` referencia una Template Page. Las Template Pages también son el origen del page-level cloning via `POST /api/v1/pages/from-template`.
_Avoid_: plantilla de página, schema

### Card Shape
Forma visual que toma un Bloque cuando activa una Template. Se declara como propiedad `card-shape:: <shape>` en la Template Page, no en el Bloque. Shapes en V1: `reference` (card plana con metas y acciones), `content` (card colapsable), `inline` (bloque normal con decoración). El renderizado es data-driven: un `CardRenderer` genérico interpreta el shape, no hay componentes React hardcodeados por tipo.
_Avoid_: tipo de card, layout predefinido

### Card Renderer
Componente del frontend que interpreta el `card-shape::` de la Template Page activada por un Bloque y produce el HTML correspondiente. Es data-driven: recibe el shape y los metas del bloque, no conoce tipos hardcodeados. Reemplaza los anteriores `ReferenceCard` y `ContentCard`.
_Avoid_: componente de card, tipo de bloque

### DSL (Query DSL)
Lenguaje de consultas tipo `(and (task TODO) (priority A))`. Base compartida entre UI y MCP. El MCP expone un superconjunto con `analyze`, `aggregate`, `stats`, `group_by`.
_Avoid_: query language, Datalog, SQL

### Propiedad
Par clave-valor tipada en un bloque o página (`status:: draft`, `priority:: A`). Las propiedades son el mecanismo de colaboración: el usuario y el agente negocian estado y contexto mediante propiedades custom.
_Avoid_: metadato, atributo, tag

### Propuesta
Contenido creado por un agente via MCP, marcado con `created_by:: agent::nombre`. El usuario acepta, rechaza o solicita revisión. El workflow es por convención, no impuesto por Quilt.
_Avoid_: sugerencia, recomendación, output

### Análisis estructural
Motor en Rust dentro de Quilt: decay detection, orphan detection, graph connectivity, similitud estructural, template expansion. Responde "qué hay y cómo está conectado".
_Avoid_: IA, NLP, machine learning

### Análisis semántico
Comprensión de significado y conexiones conceptuales. Lo hacen los agentes externos con sus modelos. Quilt no lo implementa.
_Avoid_: análisis de texto, NLP interno

### Decay
Bloques o páginas sin actividad reciente, detectados por `updated_at`. Expuesto via MCP para que el agente actúe.
_Avoid_: stale, outdated, viejo

## Relationships

- Un **Grafo** contiene muchas **Páginas**
- Una **Página** contiene muchos **Bloques** en jerarquía
- Un **Bloque** puede tener **Propiedades**, **Refs** a otros bloques/páginas, y un **Template** asociado
- Un **Template** referencia una **Template Page** (prefijo `template/`) que define su estructura y Card Shape
- Una **Template Page** declara su **Card Shape** (`card-shape::`) que el **Card Renderer** interpreta
- El **MCP Server** expone operaciones sobre el **Grafo** a **Agentes** externos
- Los **Agentes** crean **Propuestas** (bloques con `created_by:: agent::*`)
- El **Análisis estructural** provee datos al **MCP Server** para que los **Agentes** entiendan el **Grafo**
- Las **Propiedades** son el mecanismo de comunicación entre **Usuario** y **Agente**
- El **DSL** es compartido: la UI usa el base, el MCP usa el superconjunto

## Flagged ambiguities

- "Cognitive features": en `quilt-ui-workflows.md` se describe como UI. Se redefine como capacidades expuestas via MCP, integradas en la UI Logseq como paneles/secciones, no como vistas separadas.
- "Tauri": descartado. Eliminar toda referencia. UI es Leptos 0.7 CSR en browser.
- "logseq_*": prefijo obsoleto. Usar `quilt_*` en todo MCP tool naming.
- "Editor": usar **Outliner** cuando hablamos de la experiencia principal del usuario. Reservar "motor de edición" para la capa técnica que renderiza y captura input dentro de un **Bloque**.

## Example dialogue

> **Dev**: El agente encontró 3 bloques en decay en el grafo del usuario.
>
> **Domain expert**: Bien, el motor de análisis estructural los detectó por `updated_at`. El agente los lee via `quilt_analyze` y crea propuestas con `created_by:: agent::claude` y `status:: proposed`. El usuario ve las propuestas en el outliner y decide.
>
> **Dev**: ¿Y si el usuario quiere que el agente conecte dos notas que hablan de lo mismo?
>
> **Domain expert**: Ese es análisis semántico — lo hace el agente, no Quilt. Quilt le da los bloques via MCP, el agente determina la conexión conceptual, y propone crear la ref con una propuesta. El usuario aprueba.
