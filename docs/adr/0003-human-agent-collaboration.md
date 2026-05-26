# ADR-0003: Colaboración humano-agente mediante propiedades y convención

Status: accepted

El contenido creado por agentes via MCP se marca con metadatos (`created_by:: agent::claude`). No hay estados fijos de workflow — el usuario y el agente negocian mediante propiedades custom (ej: `status:: proposed`, `status:: in-review`, `status:: accepted`). Quilt solo provee las primitivas (CRUD, propiedades, queries). Los templates actúan como contrato: definen estructura y tipos que el agente debe respetar. El usuario marca bloques con comentarios/propiedades, el agente los lee via MCP y actúa. Los comentarios son bloques hijos con propiedades de tipo comment, no un sistema de chat separado.

## Considered Options

1. **Estados fijos** (proposed → accepted → rejected) — rejected: rígido, no cubre todos los flujos
2. **Sin workflow** (agente escribe directo) — rejected: el usuario pierde control
3. **Convención sobre propiedades** — accepted: flexible, extensible, consistente con el modelo Logseq

## Consequences

- El MCP debe exponer tools para leer/crear bloques con propiedades arbitarias
- Las queries DSL deben poder filtrar por `created_by`, `status`, y cualquier propiedad custom
- Los templates definen la "interface" entre agente y usuario: structure + types
