# Quilt — AI-first Knowledge Graph

PKM (Personal Knowledge Management) con UI estilo Logseq que expone un MCP server para que agentes AI externos colaboren con el usuario sobre el grafo de conocimiento. Quilt no tiene IA interna.

## Language

### Grafo
El conocimiento del usuario almacenado en SQLite: páginas, bloques jerárquicos, referencias, propiedades y tags. Un solo grafo por usuario.
_Avoid_: base de datos, knowledge base, vault

### Bloque
Unidad atómica de contenido en el outliner. Tiene UUID inmutable, contenido markdown, propiedades tipadas, refs a otros bloques/páginas, y posición jerárquica (parent, order, level).
_Avoid_: nodo, item, entrada

### Contenido de Bloque
Texto markdown editable que pertenece directamente al Bloque. No es una Propiedad. Quilt puede derivar Properties visibles o invisibles a partir de la sintaxis del contenido —por ejemplo heading-level desde `#`, links desde markdown links, o media metadata desde una URL— pero el texto visible sigue siendo contenido del Bloque.
_Avoid_: text property, content como metadata, duplicar texto en properties

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
Página con nombre prefijado por `template/` que define la estructura completa de una página: properties (schema con tipos y layout), estilos (`card-shape::`, `icon::`, `cssclass::`), y contenido (bloques hijos que pueden incluir Acciones, Queries embebidas, Views y secciones). Es el contrato completo entre agente y usuario — no solo estructura visual, sino comportamiento. Las queries embebidas usan variables `{{this.property}}` que se resuelven con las properties de la página creada.
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
Par clave-valor tipada en un bloque o página (`status:: draft`, `priority:: A`). Las propiedades son la fuente canónica de la información semántica, estructural y de proyección de un Bloque: estado, prioridad, fechas, media, tipo visual y metadata viven como properties, no como campos especiales duplicados. Pueden ser visibles inline, visibles solo en panel, o internas/sistema. Tipos soportados: Text, Number, Date, DateTime, Url, Checkbox, Select, Multi-select, Relation, Rollup, Formula, Node (page-ref). La forma en que una property se muestra, se fusiona o puede editarse se expresa como Configuración de Propiedad dentro del grafo.
_Avoid_: metadato, atributo, tag

### Configuración de Propiedad
Properties que describen otra Propiedad: tipo, visibilidad, mutabilidad, política de fusión y preferencia de representación. No introduce primitivas nuevas fuera del grafo; si Quilt necesita expresar que una property se ve inline, en panel, como indicador, como preview o como metadata de sistema, esa decisión vive como properties sobre la configuración/esquema de esa Propiedad.
_Avoid_: slot visual externo, metadata fuera del grafo, registry UI como fuente de verdad

### Visibilidad de Propiedad
Regla que define dónde se muestra una Propiedad: `inline` junto al Bloque, `panel` solo en el panel derecho, `system` oculta por defecto pero visible con “show system”, o `hidden` reservada para agentes/MCP/debug. En modo edición de Bloque, las properties del Bloque son visibles como parte de la superficie de edición, respetando su mutabilidad.
_Avoid_: display flag genérico, CSS visibility

### Mutabilidad de Propiedad
Regla que define si una Propiedad puede editarse desde la UI. Una property mutable puede cambiarse en el panel o en modo edición; una property inmutable se muestra como lectura y solo cambia por el sistema, importadores o reglas explícitas.
_Avoid_: disabled input como decisión de dominio, readonly accidental

### Proyección de Bloque
Forma contextual de visualizar un Bloque derivada de sus Properties. Todo Bloque conserva una Superficie Base universal —texto enriquecido, enlaces y properties visibles— y las proyecciones especializadas componen capas de visualización encima de esa base. Proyecciones como Task, Media, Query, Card o Annotation se activan leyendo properties del Bloque, no por tipos hardcodeados ni columnas especiales, y no reemplazan destructivamente el contenido del Bloque.
_Avoid_: tipo de bloque visual, componente especial, renderer hardcodeado, reemplazo del bloque

### Superficie Base de Bloque
Visualización universal de cualquier Bloque: contenido textual enriquecido, enlaces, hijos y properties visibles según su visibilidad y contexto de edición. Existe siempre, incluso cuando aplican proyecciones especializadas. Las proyecciones enriquecen o insertan representación adicional dentro de esta superficie, pero no eliminan ni reemplazan el texto ni las properties del Bloque.
_Avoid_: fallback pobre, renderer por defecto como caso residual, vista sin properties

### Visualización Genérica de Texto
Modo seguro y universal de mostrar un Bloque como texto enriquecido con sus enlaces, hijos y properties visibles. Es el fallback obligatorio cuando ninguna proyección especializada aplica, cuando hay ambigüedad, o cuando las properties derivadas y aplicadas producen contratos incompatibles. Nunca intenta inferir intención destructivamente.
_Avoid_: error visual, pantalla vacía, elegir una proyección dudosa

### Contrato de Proyección
Conjunto principalmente declarativo de Properties requeridas y condiciones que activan una Proyección de Bloque, con un escape hatch de código para casos complejos. Por ejemplo, TaskProjection requiere `type:: task` y un `status::` compatible; VideoProjection requiere `type:: media` y `media-type:: video`. Ninguna property aislada debería activar una proyección sin contexto suficiente.
_Avoid_: if hardcodeado, renderer selector manual, type switch

### Preferencia de Proyección
Property visual que expresa cómo debe resolverse la Proyección de Bloque. `projection:: auto` indica que Quilt debe elegir la mejor proyección compatible según los Contratos de Proyección disponibles y caer a la Proyección por Defecto si ninguna aplica. En bloques comunes, ausencia de `projection` equivale a `projection:: auto`; los Property Presets deben materializar `projection:: auto` explícitamente. Un valor específico como `projection:: task` expresa una preferencia explícita, pero no debe activar una proyección si su contrato no se cumple.
_Avoid_: renderer forzado, componente visual directo, default como contrato

### Property Preset
Conjunto nombrado de Properties que se aplica a un Bloque como atajo de creación o transformación. Un slash command aplica un Property Preset; no elige directamente la Proyección de Bloque. La proyección se deriva después leyendo las Properties resultantes.
_Avoid_: comando de render, tipo visual, componente

### Property Patch
Operación no destructiva que aplica, agrega o ajusta un subconjunto de Properties en un Bloque. Conserva el texto, los hijos y las properties no relacionadas del Bloque. Si una property existente contradice el contrato del preset o de la proyección esperada, el patch debe fallar o pedir confirmación en vez de borrar o convertir información silenciosamente.
_Avoid_: convertir bloque, reemplazo total, borrar contenido, comando destructivo

### Canonización de Entrada
Proceso que transforma input del usuario —Markdown, paste, slash command, picker, API o MCP— en Contenido de Bloque más Properties derivadas o aplicadas. El texto queda en el Bloque; las Properties capturan semántica, estructura, enlaces, proyección o metadata detectada. Slash commands son automatizaciones de canonización que aplican texto y/o Property Patches.
_Avoid_: comando especial, parser visual, texto como property

### Canonización Markdown
Caso de Canonización de Entrada donde sintaxis Markdown en el Contenido de Bloque deriva Properties. En V1, headings derivan `heading-level` y `block-role:: heading`; links markdown derivan `link`, `link-label` y `link-kind`; embeds markdown derivan `embed-url` y `embed-kind`; refs `[[Page]]` derivan `page-ref`; refs `((block-id))` derivan `block-ref`; prefijos TODO/DOING/DONE derivan `type:: task`, `status` y `projection:: auto`. El resto del Markdown permanece solo como Contenido de Bloque.
_Avoid_: markdown como renderer solamente, duplicar texto, parser sin modelo canónico

### Propiedad Derivada
Property calculada desde el Contenido de Bloque por Canonización de Entrada. Se materializa en el Bloque como `system`, queryable e inmutable desde la UI. Se modifica editando la fuente que la produjo, no editando la property directamente; si la sintaxis fuente cambia o desaparece, Quilt regenera o elimina la property derivada. Por ejemplo, `# Título` deriva `heading-level:: 1`; cambiarlo a `## Título` actualiza la property derivada.
_Avoid_: property editable duplicando markdown, segunda fuente de verdad, override manual

### Conflicto de Proyección
Situación donde Properties derivadas y aplicadas coexisten pero activan contratos de proyección incompatibles o ambiguos. Quilt no borra ni convierte información automáticamente: conserva Contenido de Bloque y Properties, materializa el conflicto como properties de sistema (`projection-conflict`, `projection-conflict-reason`, `projection-conflict-candidates`), y muestra la Visualización Genérica de Texto hasta que el usuario o una regla explícita resuelva la intención.
_Avoid_: resolver borrando, inferencia destructiva, elección silenciosa de proyección

### Política de Fusión de Propiedad
Regla que define qué ocurre cuando un Property Patch toca una property que ya existe. Las políticas posibles incluyen: set-if-missing, overwrite, append, union, reject-on-conflict y ask-on-conflict. La política se define por property/preset/contexto; el default conceptual es preservar información y escalar colisiones reales.
_Avoid_: overwrite por defecto, last-write-wins ciego, merge implícito

### Foco de Bloque
Property que marca que un Bloque pertenece al foco actual del usuario sin cambiar su estado de tarea. `focus:: now` significa “este Bloque debe aparecer en el foco actual”. Es independiente de `status:: todo/doing/waiting/done` y conceptualmente single-value.
_Avoid_: estado de tarea, en ejecución, vista NOW como entidad separada

### Slash Command
Atajo de entrada del Outliner para aplicar un Property Preset al Bloque actual. Ejemplos: `/TODO` aplica `type:: task` y `status:: todo`; `/Video` aplica properties de media. No contiene lógica de render propia.
_Avoid_: render command, bloque especial

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

### Rol
Interpretación semántica de un Bloque basada en sus Properties. No es un tipo de entidad — es un Bloque con properties que le dan significado especial. Los roles se descubren leyendo `type:: <rol>`. Ejemplos: `annotation`, `query`, `action`, `view`, `link`, `comment`, `task`, `agent-run`, `insight`.
_Avoid_: tipo especial, entidad, objeto

### Annotation
Bloque hijo con `type:: annotation` que marca un fragmento de texto dentro de su bloque padre. Properties: `target-offset`, `target-length`, `target-text`, `resolved`, `created_by`. El renderizador subraya el rango en amarillo. Navegable desde panel lateral y sidebar. Accesible por agentes via MCP para leer y crear.
_Avoid_: comment, highlight, marca

### Link (rol)
Bloque con `type:: link` que representa una relación tipada entre dos entidades del grafo. Properties: `source`, `target`, `verb` (tipo de relación), `weight` (fuerza 0.0-1.0), `confidence` (certeza), `decay` (debilitamiento temporal). Se auto-materializa a partir de referencias inline (`[[page]]`, `((uuid))`) y pueden ser creados explícitamente por usuarios y agentes. Los pesos se calculan a partir de señales del grafo: frecuencia de co-acceso, properties compartidas, annotations compartidas, tiempo sin acceso.
_Avoid_: conexión, arista, edge (salvo contexto técnico)

### Acción
Bloque con `type:: action` que define una operación ejecutable. Tipos: `prompt` (texto enviado al agente AI via MCP), `query` (ejecuta DSL), `link` (navega), `set-property` (modifica property en otro bloque/página). Futuro: `script` (JavaScript). Se renderizan como botones en la UI.
_Avoid_: botón, trigger, automation

### Query embebida
Bloque con `type:: query` y property `dsl:: (and ...)` que define una consulta parametrizable. La query usa variables `{{this.property}}` que se resuelven con las properties de la página actual. Se renderiza como vista (tabla, kanban, timeline, lista, grafo) según property `display::`. Un bloque `type:: view` puede referenciar este query via `data-source::` para crear vistas guardadas reutilizables.
_Avoid_: widget, componente, saved search

### Grafo pesado (Weighted Graph)
El grafo de Quilt donde cada Link tiene un `weight` numérico (0.0-1.0) que representa la fuerza de la relación. Los pesos se calculan automáticamente por el motor de análisis estructural a partir de señales: referencias inline (+0.3), links explícitos con verb (+0.5), co-acceso (+0.1), properties compartidas (+0.2), annotations compartidas (+0.3), decay temporal (-0.1/mes). Los agentes pueden consultar subgrafos filtrados por peso mínimo.
_Avoid_: graph neural network, embedding, vector

### AgentRun
Bloque con `type:: agent-run` que representa una ejecución atómica de un agente externo.
Propiedades: `agent::`, `model::`, `run-status::` (Queued|Running|Completed|Failed|Cancelled),
`started-at::`, `completed-at::`, `context-page::`, `summary::`, `blocks-modified::`, `error::`.
El ciclo de vida se modela con `run-status::` (mismo patrón que `status:: todo/done`).
Consultable por DSL. No es una entidad de dominio separada — es un rol de bloque.
_Avoid_: ejecución, batch, sesión de agente, run entity

### SavedView
Bloque con `type:: view` que compone una referencia a un bloque Query (`data-source::`)
con configuración de renderizado (`view-type::`, `group-by::`, `sort::`) y metadata
(`view-name::`, `view-icon::`, `view-pinned::`). Múltiples views pueden referenciar el mismo
query (misma data, distintos renderers). No es una entidad separada — es un rol de bloque.
_Avoid_: vista guardada como entidad, saved_views table

### DashboardLayout
Preset de paneles persistible a nivel workspace. Define qué paneles son visibles y su
disposición. No es un "modo de trabajo" — es configuración de layout del frontend.
Sin entidad en el dominio de Rust.
_Avoid_: work mode, modo, vista de trabajo, layout mode

## Frontend Concepts

### CommandRegistry
React context en `quilt-ui/src/features/command-center/` que registra comandos ejecutables.
Interface TypeScript: `Command { id, label, category, shortcut?, priority, target, execute }`.
`target: 'client' | 'server'` permite dispatch MCP híbrido. Activado por `Cmd+Shift+K`.
Separado del SearchModal (`Cmd+K`). Los comandos server-side van por `quilt_execute_command`.
_Avoid_: command palette, launcher, spotlight, god modal

### ViewContainer
Componente React page-level que maneja el layout de una vista guardada. Interpreta el bloque
`type:: view` y delega al LayoutEngine correspondiente según `view-type::`. No confundir con
CardRenderer (block-level, formatea un solo bloque según `card-shape::`).
_Avoid_: view renderer, display container, card container

### Cognitive* (familia de paneles)
Namespace `cognitivo::` (ADR-0001). Tres paneles integrados en la UI Logseq:
- AgentActivityFeed: actividad de agentes (reemplaza AgentActivityPanel)
- StructuralGraph: topología, conectividad, decay, orphans (Quilt lo calcula)
- SemanticInsight: significado de conexiones (agente externo lo provee, Quilt lo muestra)
_Avoid_: serendipity feed, agent workbench, connection feed

### StrategySelector
Trait en `quilt-core` (WASM-compatible): `fn select(features, scorer, portfolio) -> Vec<RankedAction>`.
Trait `StrategyScorer` separado: `fn score(action, features) -> f32`.
Tipos: `ContextFeatures` (ContentShape + GraphShape + SchemaShape + UsageContext),
`RankedAction` (action_id, label, kind, score, rationale).
Phase 1: reglas determinísticas, 6-8 portfolio actions, top 3 como hints. Sin telemetría.
Expuesto via WASM (frontend) y `quilt_strategy_select` MCP tool (agentes).
_Avoid_: ML, recomendador, predictor, SUNNY engine

## Relationships

- Un **Grafo** contiene muchas **Páginas**
- Una **Página** contiene muchos **Bloques** en jerarquía
- Un **Bloque** puede tener **Propiedades**, **Refs** a otros bloques/páginas, y un **Template** asociado
- Un **Template** referencia una **Template Page** (prefijo `template/`) que define su estructura y Card Shape
- Una **Template Page** declara su **Card Shape** (`card-shape::`) que el **Card Renderer** interpreta
- Una **Template Page** define el layout de **Properties** (header/inline/panel), estilos (`cssclass::`, `icon::`), **Acciones**, **Queries embebidas** y **Views** como bloques hijos
- El **MCP Server** expone operaciones sobre el **Grafo** a **Agentes** externos
- Los **Agentes** crean **Propuestas** (bloques con `created_by:: agent::*`)
- El **Análisis estructural** provee datos al **MCP Server** para que los **Agentes** entiendan el **Grafo**
- Las **Propiedades** son el mecanismo de comunicación entre **Usuario** y **Agente**
- El **DSL** es compartido: la UI usa el base, el MCP usa el superconjunto
- Un **Rol** es una interpretación de un **Bloque** según sus **Properties** (`type:: <rol>`)
- Una **Annotation** es un **Bloque** (rol) que marca un fragmento de texto en su padre
- Un **Link** es un **Bloque** (rol) con peso que conecta dos entidades del **Grafo pesado**
- Una **Acción** es un **Bloque** (rol) que define una operación ejecutable (prompt, query, set-property)
- Una **Query embebida** es un **Bloque** (rol) con DSL parametrizable por **Properties** de la página actual
- Una **View** es un **Bloque** (rol) con `type:: view` que define una vista guardada sobre datos del **Grafo pesado**. Compone una referencia a un **Bloque Query** via `data-source::`. Propiedades: `view-type::` (table|kanban|calendar|list|graph|cards|timeline), `data-source::`, `view-name::`, `view-icon::`, `view-pinned::`, `group-by::`, `sort::`. El **ViewContainer** (frontend) interpreta el bloque view y delega al **LayoutEngine** correspondiente.
- Un **SavedView** es un **Bloque** (rol) con múltiples views sobre el mismo **Bloque Query**

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
