# ADR-0005: Sin Tauri — Leptos 0.8 CSR como UI, eliminación completa de Tauri

Status: accepted

Quilt no usa Tauri. La UI se construye con Leptos 0.8 en modo CSR (client-side rendering), compilada a WASM, ejecutándose en browser. Todo el código relacionado con Tauri en `quilt-platform/src/tauri.rs`, `quilt-platform/src/tauri/`, y la dependencia en `Cargo.toml` se elimina. El crate `quilt-platform` se reenfoca en CLI y WASM shell.

## Considered Options

1. **Tauri** — rejected: el usuario lo descartó explícitamente
2. **Leptos SSR** — rejected: añade complejidad de servidor, CSR es suficiente para una app local con backend SQLite
3. **Leptos CSR en browser** — accepted: simple, WASM nativo, consistente con el stack Rust

## Consequences

- Eliminar `quilt-platform/src/tauri.rs`, `quilt-platform/src/tauri/`
- Eliminar dependencia `tauri` de workspace `Cargo.toml`
- `quilt-ui/src/lib.rs` debe eliminar referencia a "Tauri IPC"
- El backend SQLite se accede via el servidor MCP (HTTP/WebSocket), no via IPC de Tauri
