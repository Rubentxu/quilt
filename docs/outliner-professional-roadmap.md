# Roadmap Técnico — Outliner Profesional

Objetivo: llevar Quilt desde el outliner actual a un **Outliner profesional** capaz de competir y superar a Logseq, preservando el modelo de dominio (**Grafo → Página → Bloque → Propiedad**) y manteniendo Quilt estrictamente **MCP-first**.

## Fase 0 — Preparación y blindaje

### Objetivos
- Consolidar decisiones ya tomadas en docs y ADRs.
- Identificar puntos exactos del código actual a reemplazar o refactorizar.
- Preparar puntos de extensión sin romper el flujo actual.

### Dependencias
- `docs/adr/0006-outliner-engine-over-domain-model.md`
- `docs/outliner-professional-baseline.md`

### Riesgos
- Empezar a implementar sin fijar fronteras entre dominio y motor.

### Criterio de salida
- Frontera dominio/motor clara.
- Estrategia de migración definida.

### No hacer todavía
- No añadir features aisladas sobre `contenteditable` si van a quedar obsoletas.

## Fase 1 — Motor serio por Bloque + parser unificado

### Objetivos
- Sustituir `contenteditable` por un motor serio por **Bloque**.
- Introducir parser incremental unificado para `property:: value`, `[[Página]]`, `((Bloque))`, `#tag`.

### Dependencias
- Fase 0

### Riesgos
- Acoplar demasiado el motor nuevo al estado textual interno.
- Repetir la lógica semántica entre editor y outliner.

### Criterio de salida
- Cada bloque se edita mediante adaptador nuevo.
- Parser devuelve semántica estructurada sin perder sintaxis visible.

### No hacer todavía
- No implementar aún slash commands complejos ni drag & drop.

## Fase 2 — Refs, tags y properties con autocomplete

### Objetivos
- `[[Página]]` con autocomplete.
- `((Bloque))` con autocomplete.
- `#tag` y `tags::` normalizados a `tags`.
- Propiedades v1 con autocomplete tipado.

### Dependencias
- Fase 1

### Riesgos
- Resolver autocomplete como hack del editor en vez de intención del Outliner.

### Criterio de salida
- Parser + autocomplete + normalización funcionando de punta a punta.

### No hacer todavía
- No meter properties secundarias ni workflows arbitrarios.

## Fase 3 — Undo/Redo unificado por intención

### Objetivos
- Introducir historia unificada de operaciones del Outliner.
- Soportar undo/redo tanto para texto como para estructura y propiedades.

### Dependencias
- Fases 1 y 2

### Riesgos
- Separar historia textual y estructural.

### Criterio de salida
- `Mod+Z` y `Mod+Shift+Z` revierten intenciones del usuario de forma coherente.

### No hacer todavía
- No optimizar microagrupaciones avanzadas antes de tener el modelo correcto.

## Fase 4 — Navegación completa por teclado

### Objetivos
- Navegación entre bloques estilo Logseq.
- Atajos de edición y estructurales canónicos.
- `Mod+Enter` para `TODO → DOING → DONE → TODO`.

### Dependencias
- Fases 1 y 3

### Riesgos
- Mezclar comportamientos del motor con operaciones del Outliner.

### Criterio de salida
- El outliner se puede usar de forma keyboard-first.

### No hacer todavía
- No introducir todavía combinaciones marginales o avanzadas no esenciales.

## Fase 5 — Sidebar derecho funcional

### Objetivos
- Backlinks reales.
- Unlinked references.
- Base para paneles múltiples.

### Dependencias
- Fase 2

### Riesgos
- Dejar las refs solo como decoración textual.

### Criterio de salida
- El sidebar derecho aporta contexto estructural real.

### No hacer todavía
- No convertirlo aún en un panel “AI” ni meter lógica ajena al dominio.

## Fase 6 — Slash commands

### Objetivos
- Sistema `/` alineado con la semántica ya implementada.
- Comandos sobre status, priority, fechas, refs y templates.

### Dependencias
- Fases 1 y 2

### Riesgos
- Construir un menú rico encima de semántica inestable.

### Criterio de salida
- Slash commands útiles y coherentes con propiedades/refs del dominio.

### No hacer todavía
- No meter comandos cosméticos por delante de comandos estructurales.

## Fase 7 — Propiedades inline fuertes

### Objetivos
- Mejor visualización y edición inline de propiedades v1.
- Validación tipada y feedback inmediato.

### Dependencias
- Fases 2 y 6

### Riesgos
- Convertir propiedades en badges visuales sin semántica fuerte.

### Criterio de salida
- `status`, `priority`, `scheduled`, `deadline`, `tags`, `template`, `created_by` se editan inline con robustez.

### No hacer todavía
- No abrir todavía un panel lateral de properties; se acordó inline-only.

## Fase 8 — Drag & drop

### Objetivos
- Reordenamiento de bloques y cambios de jerarquía con feedback visual.

### Dependencias
- Fases 1, 3 y 4

### Riesgos
- Romper la historia de undo/redo o la integridad del árbol.

### Criterio de salida
- Drag entre siblings y sobre bullets para hacer children.

### No hacer todavía
- No mezclar aún drag de bloques con features avanzadas de multi-panel.

## Fase 9 — Journals funcionales

### Objetivos
- Navegación de journals fluida.
- Creación automática por fecha.
- Calendario como capa secundaria.

### Dependencias
- Fases 1 y 4

### Riesgos
- Tratar `journal` como propiedad general de bloque en vez de semántica de página.

### Criterio de salida
- Journals navegables y consistentes con el resto del Outliner.

### No hacer todavía
- No mezclar journals con workflows cognitivos o paneles no esenciales.

## Regla de priorización

**Primero semántica, luego confianza, luego velocidad**.

- Semántica: motor, parser, refs, tags, properties.
- Confianza: undo/redo.
- Velocidad: teclado, slash, drag & drop.
