# ADR-DRAFT: AgentRun como rol de bloque `type:: agent-run`

Status: draft

## Context

La sesión de auto-grill (Q005-P1 + Q012-P2, 2026-06-07) debatió si las ejecuciones de agentes AI externos deben modelarse como una entidad de dominio separada o como una convención de propiedades sobre bloques existentes.

El fork fue el más disputado de la sesión: Q005-P1 fue rechazado por invocar YAGNI sin modelar, y Q012-P2 fue la peor violación de protocolo registrada (el Proxy resucitó textualmente la respuesta rechazada de Q005-P1).

ADR-0003 define colaboración humano-agente por convención de propiedades (`created_by:: agent::claude`, `status:: proposed`), pero solo cubre **proveniencia a nivel de bloque**, no **ejecución a nivel de run**.

El fork fue resuelto por decisión del arquitecto tras revisar la evidencia de 14 ciclos de grill.

## Decision

**AgentRun es un rol de bloque con `type:: agent-run` y propiedades que modelan el ciclo de vida. NO es una entidad de dominio separada.**

### Modelo de propiedades

| Property | Type | Required | Purpose |
|----------|------|----------|---------|
| `type::` | role | Sí | `agent-run` |
| `agent::` | string | Sí | Agente (e.g. `claude`, `gemini`) |
| `model::` | string | No | Modelo usado |
| `run-status::` | select | Sí | `Queued` \| `Running` \| `Completed` \| `Failed` \| `Cancelled` |
| `started-at::` | datetime | No | Inicio de ejecución |
| `completed-at::` | datetime | No | Fin de ejecución |
| `context-page::` | page-ref | No | Página de contexto |
| `summary::` | text | No | Resumen del resultado |
| `blocks-modified::` | block-ref[] | No | UUIDs de bloques modificados |
| `error::` | text | No | Mensaje de error si falló |

### Ciclo de vida

```
Queued → Running → Completed
                 → Failed
                 → Cancelled
```

Modelado via `run-status::` — mismo patrón que `status:: todo/done` en tareas.

### Consultabilidad

Ejemplos de queries DSL:
```lisp
;; Todos los runs fallidos en la última hora
(and (type agent-run) (run-status Failed) (completed-at (> "2026-06-07T09:00:00Z")))

;; Último run de Claude
(and (type agent-run) (agent claude)) order-by started-at desc limit 1

;; Bloques modificados por un run específico
blocks-modified contains "<uuid>"
```

## Considered Options

1. **Entidad AgentRun separada** (rechazado) — requiere nueva tabla SQLite, migración, repositorio. Viola el principio "todo es un bloque" del sistema de roles de Quilt.
2. **Solo convención ADR-0003 sin entidad de run** (rechazado) — ADR-0003 cubre proveniencia de bloques, no ejecución de runs. Sin run explícito, no hay trazabilidad de operaciones atómicas.
3. **Rol de bloque `type:: agent-run`** — aceptado: modela ciclo de vida con properties, consultable por DSL, sin migración, consistente con el sistema de roles.

## Consequences

- No se crea tabla `agent_runs` en SQLite
- Las queries sobre runs usan la misma infraestructura DSL que cualquier bloque
- `blocks-modified::` permite navegar del run a los bloques afectados
- Los agentes crean el bloque agent-run al iniciar y actualizan `run-status::` al finalizar
- MCP tool `quilt_create_agent_run` o el agente usa `quilt_create_block` con las properties correspondientes

## References

- Q005-P1 y Q012-P2 (auto-grill 2026-06-07)
- ADR-0003: Colaboración humano-agente por convención de propiedades
- CONTEXT.md: Rol, Bloque, Propiedad
- Resolución del arquitecto (2026-06-07): AgentRun es rol de bloque
