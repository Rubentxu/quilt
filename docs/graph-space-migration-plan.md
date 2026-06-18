# Graph Space migration plan

Este documento traduce la ADR-0030 a trabajo técnico concreto sobre el código actual.

## Resultado buscado

Quilt debe comportarse como una aplicación de **Graph Space** local-first con:

- `quilt.db` canónica en `<graph-root>/.quilt/quilt.db`
- Journal de hoy como entrada estable
- selector de Graph solo como fallback
- panel derecho contextual como superficie operativa principal secundaria

## Quick path

1. Unificar bootstrap de Graph en server, CLI y MCP.
2. Persistir estado global (`last_opened_graph`, recientes, layout global) fuera del Graph.
3. Validar Graphs de forma explícita en el arranque.
4. Mover la entrada de UI al Journal de hoy.
5. Reubicar Morning Briefing como helper colapsable de Journal vacío/nuevo.
6. Diseñar contrato del panel derecho contextual.

## Fases

## Fase 1 — Unificar bootstrap del Graph

### Objetivo

Eliminar el modelo viejo de `quilt.db` suelto.

### Cambios

| Superficie | Cambio |
|-----------|--------|
| Server | reutilizar un único bootstrap de Graph Space |
| CLI | sustituir `--db-path` por `--graph-dir` |
| MCP | sustituir `QUILT_DB_PATH` por `QUILT_GRAPH_DIR` |
| README | dejar de documentar apertura por `.db` |

### Archivos implicados

- `crates/quilt-server/src/main.rs`
- `crates/quilt-platform/src/init.rs`
- `crates/quilt-platform/src/cli.rs`
- `crates/quilt-bin/src/mcp_main.rs`
- `README.md`

## Fase 2 — Estado global de aplicación

### Objetivo

Persistir fuera del Graph:

- `last_opened_graph`
- recent graphs
- visibilidad del panel derecho

### Cambios

- dejar de depender solo de `last_opened_graph` en memoria
- introducir store global de aplicación

### Archivos candidatos

- `crates/quilt-server/src/state.rs`
- nuevo módulo de app state global en `crates/quilt-platform/` o `crates/quilt-infrastructure/`

## Fase 3 — Validación explícita del Graph

### Objetivo

No abrir ni recrear silenciosamente graphs inválidos.

### Validaciones mínimas

- existe el directorio del Graph
- existe `.quilt/`
- existe `quilt.db`
- la base abre correctamente
- el schema mínimo es válido

### Archivos candidatos

- `crates/quilt-platform/src/init.rs`
- `crates/quilt-server/src/main.rs`
- nuevo módulo `graph_validation.rs`

## Fase 4 — Selector de Graph y flujo de arranque UI

### Objetivo

Arranque determinista:

- `last_opened_graph` válido → abrir directo
- no válido → selector

### Alcance del selector

- abrir Graph existente
- crear Graph en directorio elegido

### Fuera de alcance del selector

- edición rica de identidad del Graph Space

### Archivos candidatos

- `quilt-ui/src/router.tsx`
- nueva página `GraphSelectorPage.tsx`
- `quilt-ui/src/App.tsx`

## Fase 5 — Journal-first entry

### Objetivo

Todo Graph abre en el Journal de hoy.

### Reglas

- crear Journal de hoy si falta
- no restaurar última ruta interna
- soportar anterior/siguiente/calendario

### Archivos candidatos

- `quilt-ui/src/pages/JournalPage.tsx`
- navegación/rutas relacionadas

## Fase 6 — Morning Briefing correcto

### Objetivo

Convertir Morning Briefing en helper contextual, no cabecera fija.

### Reglas

- visible por defecto solo para hoy si el Journal está vacío o recién creado
- siempre colapsable
- no persistente como cabecera principal

### Archivos candidatos

- `quilt-ui/src/pages/JournalPage.tsx`
- `quilt-ui/src/features/cognitive/MorningBriefing.tsx`

## Fase 7 — Graph Space metadata

### Objetivo

Dar identidad de primer nivel al Graph Space.

### Campos iniciales

- name
- icon
- description
- color/theme
- created_at
- path (derivable, opcionalmente no editable)

### Recomendación

Crear tabla dedicada en SQLite en lugar de sobrecargar `config`.

### Archivos candidatos

- migraciones SQLite
- repositorio de settings o nuevo `graph_space_repo`
- UI de configuración del Graph

## Fase 8 — Panel derecho contextual

### Objetivo

Pasar de inspector accesorio a superficie operativa contextual.

### Contrato inicial

- visible por defecto en desktop
- colapsable/ocultable
- prioridad por selección activa
- edición de properties panel-first
- una acción principal máximo con alta confianza

### Necesidades técnicas

- modelo de selección activa
- resolvedor de contexto
- ranking de acciones
- ordenamiento dinámico de secciones

## Fase 9 — Ingesta manual de recursos compatibles ✅ COMPLETO

### Objetivo

Permitir Graphs sobre carpetas reales del usuario sin autoimportación mágica.

### Reglas

- scan manual
- importar o reindexar manualmente
- sin watch en v1

## Fase 10 — Local Graph v1

### Objetivo

Entregar un grafo útil antes que espectacular.

### Alcance v1

- 2D
- local
- contextual
- profundidad 1/2/3
- clic en nodo navega al contexto
- sin filtros iniciales
- identidad visual semántica desde el inicio

## Orden recomendado

1. bootstrap del Graph
2. validación explícita
3. estado global
4. selector + arranque UI
5. journal-first entry
6. briefing contextual
7. Graph Space metadata
8. panel derecho contextual
9. ingesta manual
10. local graph v1

## Checklist

- [ ] server, CLI y MCP resuelven Graph por directorio, no por `.db` suelto
- [ ] existe estado global persistido para `last_opened_graph`
- [ ] Graph inválido no se recrea en silencio
- [ ] abrir Graph siempre entra al Journal de hoy
- [ ] Morning Briefing deja de ocupar la cabecera principal por defecto
- [ ] el panel derecho tiene contrato contextual explícito

## Next step

Abrir una épica o cambio de trabajo para ejecutar Fases 1–4 antes de tocar el local graph o rediseños más vistosos.
