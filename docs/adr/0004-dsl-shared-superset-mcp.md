# ADR-0004: DSL compartido con superconjunto para MCP

Status: accepted

El Query DSL es el mismo lenguaje base para UI y MCP: `(and (task TODO) (priority A))`. El MCP expone un superconjunto con operadores adicionales: `analyze`, `aggregate`, `stats`, `group_by`. Esto permite que el usuario defina queries en la UI que el agente pueda leer y ejecutar via MCP, y que el agente pueda usar queries más potentes cuando necesite.

## Considered Options

1. **Mismo DSL exacto** — rejected: limita lo que el agente puede pedir (sin agregación, sin estadísticas)
2. **Dos lenguajes** (DSL para UI, SQL/GraphQL para MCP) — rejected: pierde simetría usuario-agente
3. **DSL base + superconjunto MCP** — accepted: simétrico donde se pueda, potente donde se necesite

## Consequences

- `quilt-query` debe soportar el superconjunto como extensiones del parser base
- Las queries son first-class citizens: se almacenan como bloques y ambos (usuario y agente) pueden leerlas y ejecutarlas
