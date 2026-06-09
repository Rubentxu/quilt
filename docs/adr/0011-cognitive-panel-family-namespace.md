# ADR-DRAFT: Familia de paneles Cognitive* bajo namespace `cognitivo::`


## Context

El documento de investigación proponía paneles para exponer capacidades cognitivas en la UI. La sesión de auto-grill Q007-P1 propuso 4 paneles (Agent Feed, Structural Mirror, Serendipity Feed, Agent Workbench) que fueron rechazados por:
1. Violación de ADR-0001: "serendipity" es el ejemplo canónico de análisis semántico que Quilt NO implementa
2. Phantom panel: Agent Workbench sin implementación ni diseño
3. Boundary estructural/semántico sin definir

Q013-P2 corrigió el diseño con 3 paneles bajo namespace `cognitivo::` (ADR-0001).

## Decision

**Tres paneles bajo la familia Cognitive\* alineados con el namespace `cognitivo::` de ADR-0001:**

### 1. AgentActivityFeed
Reemplaza el AgentActivityPanel actual (`quilt-ui/src/features/cognitive/AgentActivityPanel.tsx`). Muestra actividad de agentes: bloques creados/modificados con `agent::` property, runs recientes, propuestas pendientes.

### 2. StructuralGraph
Topología del grafo calculada por Quilt: conectividad, decay, orphans, similitud estructural. Responde "qué hay y cómo está conectado" (análisis estructural, NO semántico). Se renderiza como grafo local con filtros por tipo de relación, peso mínimo, y profundidad.

### 3. SemanticInsight
Significado de conexiones conceptuales. Lo provee el agente externo — Quilt solo muestra. El agente escribe bloques con `type:: insight` que el panel lista. Quilt no genera insights.

### Boundary estructural/semántico

| Quién | Qué calcula | Panel |
|-------|-------------|-------|
| Quilt (Rust) | Topología, conectividad, decay, orphans, similitud | StructuralGraph |
| Agente externo | Significado, serendipity, relaciones conceptuales | SemanticInsight |

### Nombres rechazados

| Nombre propuesto | Problema | Nombre final |
|-----------------|----------|--------------|
| Serendipity Feed | Violación ADR-0001 (análisis semántico) | (eliminado) |
| Agent Workbench | Phantom panel sin implementación | (eliminado) |
| ConnectionFeed | Demasiado genérico, pierde semántica de agente | AgentActivityFeed |
| ConnectionGraph | No distingue estructural de semántico | StructuralGraph |
| ConnectionInsight | Mismo problema | SemanticInsight |

## Considered Options

1. **4 paneles con Serendipity + Workbench** (rechazado por Q007-P1) — viola ADR-0001, phantom panel
2. **Sin paneles cognitivos** (rechazado) — ADR-0002 dice que features AI se integran como paneles
3. **Cognitive\* family (3 paneles)** — aceptado: alineado con ADR-0001/0002, boundary clara

## Consequences

- Se elimina código tree_rag de quilt-analysis (verificar con `cargo check`)
- AgentActivityPanel actual se reemplaza por AgentActivityFeed
- StructuralGraph usa el motor de quilt-analysis existente
- SemanticInsight es un panel de solo lectura que lista bloques `type:: insight`
- Nombres usan el namespace `cognitivo::` de ADR-0001 consistentemente

## References

- Q007-P1 y Q013-P2 (auto-grill 2026-06-07)
- ADR-0001: No IA interna, namespace `cognitivo::`
- ADR-0002: Features AI como paneles, no vistas separadas
- `quilt-ui/src/features/cognitive/AgentActivityPanel.tsx`
- `crates/quilt-analysis/` — motor de análisis estructural
