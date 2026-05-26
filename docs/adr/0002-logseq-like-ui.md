# ADR-0002: UI base es Logseq-like — las features AI se integran como paneles, no como vistas separadas

Status: accepted

La interfaz de usuario de Quilt replica el modelo de interacción de Logseq: outliner con bloques jerárquicos, edición inline, journals diarios, sidebars (izquierdo: navegación/favoritos/recientes, derecho: backlinks/contexto), búsqueda Ctrl+K, slash commands, drag & drop, colapso de bloques, y virtual scrolling. Las features de `quilt-ui-workflows.md` (serendipity, decay monitor, auto-organize, briefings, cognitive map) NO son vistas separadas tipo `/cognitive/*`. Se integran como paneles o secciones dentro de la UI Logseq. Las vistas `/cognitive/*` actuales en `quilt-ui` se eliminan.

## Considered Options

1. **5 vistas separadas** (Daily Journal, Graph View, Focus Mode, Query Builder, Agent Room) — rejected: modelo UX mutuamente excluyente con Logseq, el usuario espera un outliner
2. **UI Logseq + features AI como vistas separadas** — rejected: fragmenta la experiencia
3. **UI Logseq como base, features AI integradas** — accepted: el usuario reconoce el paradigma, las features AI aparecen como paneles/secciones dentro de la misma interfaz

## Consequences

- Prioridad de desarrollo: outliner y editor de bloques son el primer componente UI
- Los paneles cognitive (serendipity feed, decay alerts, etc.) se añaden después del baseline funcional
- `quilt-ui/src/pages/cognitive/` se elimina o reestructura como componentes integrados
