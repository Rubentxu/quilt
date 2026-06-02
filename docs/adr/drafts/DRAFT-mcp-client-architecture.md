# DRAFT ADR: MCP Client — Browser no necesita cliente MCP

Status: draft (corrected 2026-05-30)

## Context

Quilt sigue arquitectura DDD con dos puertos de presentación en el backend Rust:
1. **REST API** (`quilt-http`, puerto :3737) — para la UI web (Leptos/React)
2. **MCP Server** (`quilt-mcp`, puerto de agentes) — para agentes AI externos

El código actual contiene `crates/quilt-ui/src/wasm/client.rs` (388 líneas), un cliente MCP WebSocket que corre en el browser. Este código es **arquitectónicamente incorrecto**: el browser no necesita un cliente MCP porque los agentes AI se conectan directamente al MCP Server del backend, no a través del browser.

## Decision

**Eliminar `wasm/client.rs`, `wasm/bindings.rs`, y `wasm/signals.rs` (~726 líneas).**

El browser React solo necesita un cliente REST (`fetch()` a `http://127.0.0.1:3737/api/v1`). El MCP Server del backend Rust (`quilt-mcp`) se mantiene intacto como puerto DDD para agentes externos.

No se introduce `@modelcontextprotocol/sdk` en el browser porque no hay caso de uso. El SDK de MCP es para agentes que se conectan al servidor, no para el browser.

## Why

1. **Separación de puertos DDD**: REST es el puerto de UI, MCP es el puerto de agentes. El browser consume REST, no MCP.
2. **Los agentes AI se conectan directamente al backend**: Claude Code, Cursor, etc. hablan MCP directo con `quilt-mcp`. El browser no es intermediario.
3. **`wasm/client.rs` nunca funcionó**: el cliente MCP en browser no tiene un MCP WebSocket endpoint en el servidor (el servidor usa stdio, no WebSocket). Son 726 líneas de código muerto.
4. **Simplificación**: borrar código que no se usa ni se necesita. La migración a React no cambia esto — el código sobraba igual.

## Consequences

- Se borran 726 líneas de `crates/quilt-ui/src/wasm/`
- El MCP Server (`quilt-mcp`) NO se modifica
- El browser React solo necesita un cliente REST (fetch nativo)
- Los agentes AI externos siguen usando MCP directo al backend
