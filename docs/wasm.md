# WASM Build Documentation

This document describes how to build the Quilt UI as WASM for browser and edge runtimes.

## Prerequisites

### 1. Install wasm-pack

wasm-pack is the recommended tool for building Rust WASM for the web.

```bash
# Via cargo (recommended)
cargo install wasm-pack

# Or via npm
npm install -g wasm-pack
```

### 2. Add WASM Target

```bash
rustup target add wasm32-unknown-unknown
```

To verify the target is installed:
```bash
rustup target list --installed | grep wasm32
```

### 3. Install wasm-bindgen (usually handled by wasm-pack)

If needed manually:
```bash
cargo install wasm-bindgen-cli
```

## Building

### Basic Build

From the project root:

```bash
wasm-pack build --target web --out-dir pkg -p quilt-ui
```

This produces:
- `pkg/quilt_ui.js` - JavaScript bindings
- `pkg/quilt_ui_bg.wasm` - WASM binary
- `pkg/quilt_ui.d.ts` - TypeScript type definitions

### Build Targets

- `--target web` - For use in browser with ES modules
- `--target nodejs` - For Node.js environments
- `--target no-modules` - For traditional script tags

### Release Build

```bash
wasm-pack build --target web --out-dir pkg -p quilt-ui --release
```

## Running in Browser

### With a Simple HTTP Server

```bash
cd pkg
python3 -m http.server 8080
# or
npx serve .
```

Then open `http://localhost:8080` in your browser.

### Integration Example

```javascript
import init, { init_mcp_client, connect_mcp_client, get_connection_state } from './pkg/quilt_ui.js';

async function main() {
    await init();

    // Initialize MCP client to your server
    init_mcp_client("ws://localhost:9100/mcp");

    // Connect
    await connect_mcp_client();

    // Check connection state
    console.log(get_connection_state()); // "Connected(0)"
}

main();
```

## Limitations

### Native Dependencies Not Available in WASM

The following crates/modules cannot be used in WASM builds:

- **tokio** - Async runtime (use `gloo-timers` or `wasm-bindgen-futures` instead)
- **reqwest** (with native features) - Use `web-sys::fetch` or a WASM-compatible HTTP client
- **git2** - Git operations (not available in browser)
- **sqlx** - Database access (use IndexedDB or a WASM-compatible storage solution)

### File System Access

Browser WASM has no native file system access. Use:
- IndexedDB for persistent storage
- File System Access API (modern browsers)
- Tauri IPC for native file operations

### Network Restrictions

- WebSocket connections must comply with browser CORS policies
- HTTP fetch is available but has restrictions on cross-origin requests

## Troubleshooting

### "Cannot find wasm32 target"

```bash
rustup target add wasm32-unknown-unknown
```

### "wasm-bindgen not found"

```bash
cargo install wasm-bindgen-cli
```

### Build succeeds but runtime panics

Check that:
1. `console_error_panic_hook` is set for better error messages
2. All dependencies use WASM-compatible alternatives
3. No direct syscalls or OS-specific code paths are hit

### Connection fails immediately

The MCP client expects a WebSocket server at the configured URL. Make sure:
1. The `quilt-mcp` server is running
2. CORS is configured on the server (if accessing from a different origin)
3. The WebSocket URL is correct

## CI/CD

To add WASM build to your CI pipeline:

```yaml
# Example GitHub Actions
- name: Build WASM
  run: |
    rustup target add wasm32-unknown-unknown
    wasm-pack build --target web --out-dir pkg -p quilt-ui --release
```

## Architecture

The WASM module is structured as:

```
crates/quilt-ui/src/wasm/
├── mod.rs        - Module exports
├── client.rs     - MCP WebSocket client (ConnectionState, McpClient, backoff)
├── bindings.rs   - wasm_bindgen exports for JS interop
└── signals.rs    - Leptos signal integration
```
