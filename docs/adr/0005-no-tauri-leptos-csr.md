# ADR-0005: No Tauri — React 19 + TypeScript CSR in browser

Status: accepted

Quilt no usa Tauri. Originalmente la UI se construía con Leptos 0.8 en modo CSR (client-side rendering), compilada a WASM. Durante el desarrollo se migró a **React 19 + TypeScript** por las siguientes razones:

- **TanStack Router**: ecosistema de routing maduro con type-safety, loaders, y navegación declarativa
- **Mejor DevEx**: tooling maduro (Vite, ESLint, TypeScript), hot module replacement instantáneo, perfilado React DevTools
- **Ecosistema maduro**: react-virtuoso (virtual scrolling), @dnd-kit (drag & drop), lucide-react (iconos), react-hot-toast (notificaciones), @tiptap/react (editor rich text)
- **Comunidad**: React tiene el ecosistema más grande, facilitando reclutamiento, bibliotecas de componentes, y soluciones a problemas comunes
- **WASM coexist**: React + WASM (via vite-plugin-wasm) funciona tan bien como Leptos + WASM; el parser inline y otras funcionalidades de quilt-core se mantienen en WASM

## Considered Options

1. **Tauri** — rejected: el usuario lo descartó explícitamente
2. **Leptos SSR** — rejected: añade complejidad de servidor, CSR es suficiente para una app local con backend SQLite
3. **Leptos CSR** — initially accepted, luego superseded por React
4. **React 19 + TypeScript CSR** — accepted: stack actual

## Consequences

- La UI es una SPA React 19 + TypeScript, compilada con Vite 6
- Se eliminó `quilt-platform/src/tauri.rs`, `quilt-platform/src/tauri/`
- Se eliminó dependencia `tauri` de workspace `Cargo.toml`
- El backend SQLite se accede via el servidor MCP (HTTP/WebSocket), no via IPC de Tauri
- Librerías clave del stack: @tanstack/react-router, @tiptap/react, react-virtuoso, @dnd-kit/core + @dnd-kit/sortable, lucide-react
- El WASM (quilt-core) se carga via vite-plugin-wasm + vite-plugin-top-level-await, con bridge React -> WASM en `WasmProvider`
- React.StrictMode activo en desarrollo para detección temprana de efectos secundarios
