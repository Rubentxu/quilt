# Spec: Trunk + Tailwind CSS 4 Setup (Change trunk-tailwind-setup)

## Overview

Set up Trunk as the build tool for `quilt-ui` with Tailwind CSS 4 properly configured. The current `index.html` is broken (references non-existent `./src/main.ts` TypeScript file) and `style.css` has no build pipeline.

## Current State

- `index.html` references `./src/main.ts` (TypeScript file that does not exist)
- `style.css` uses Tailwind v4 syntax (`@import "tailwindcss"`, `@theme`) but has no build pipeline
- No `Trunk.toml` or PostCSS configuration exists
- No npm package.json for Tailwind CLI

## Configuration Files

### crates/quilt-ui/Trunk.toml

```toml
[build]
target = "index.html"
dist = "dist"

[watch]
ignore = ["target", "node_modules", "dist"]

[build.target]
# Leptos WASM + HTML — Trunk copies index.html and styles to dist/

[build.tools]
# Tailwind CLI integration via npm scripts
```

### crates/quilt-ui/postcss.config.js

```javascript
module.exports = {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}
```

### crates/quilt-ui/package.json

```json
{
  "name": "quilt-ui",
  "private": true,
  "scripts": {
    "tailwind:watch": "tailwindcss -i style.css -o dist/style.css --watch",
    "tailwind:build": "tailwindcss -i style.css -o dist/style.css --minify",
    "dev": "npm run tailwind:build && trunk serve --open"
  },
  "dependencies": {},
  "devDependencies": {
    "@tailwindcss/postcss": "^4.0.0",
    "tailwindcss": "^4.0.0"
  }
}
```

### crates/quilt-ui/index.html (fixed)

```html
<!DOCTYPE html>
<html lang="en" class="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Quilt</title>
    <link rel="stylesheet" href="dist/style.css">
</head>
<body class="bg-base text-text">
    <div id="app"></div>
</body>
</html>
```

Changes from current:
- Removed `<script type="module" src="./src/main.ts"></script>` (broken reference)
- Changed `href="style.css"` to `href="dist/style.css"` (Trunk serves from dist)

## Dev Workflow

### Setup (one-time)

```bash
# Install Trunk
cargo install trunk

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install npm dependencies
cd crates/quilt-ui && npm install
```

### Development

**Terminal 1 — Watch Tailwind CSS:**
```bash
cd crates/quilt-ui
npm run tailwind:watch
```

**Terminal 2 — Build and serve WASM:**
```bash
cd crates/quilt-ui
trunk serve --open
```

Trunk will:
1. Build the Leptos WASM from Rust sources
2. Copy `index.html` and `dist/style.css` to `dist/`
3. Serve at `http://127.0.0.1:8080` (or next available port)

### Production Build

```bash
cd crates/quilt-ui
npm run tailwind:build   # Minified CSS
trunk build --release   # WASM build
```

Output in `dist/`:
- `index.html`
- `style.css` (processed, minified)
- `*.wasm` (Leptos WASM bundle)

## Dependencies

| Tool | Install Command | Purpose |
|------|----------------|---------|
| `trunk` | `cargo install trunk` | Build tool for WASM + HTML |
| `tailwindcss` | `npm install -D tailwindcss@4` | CSS framework |
| `@tailwindcss/postcss` | `npm install -D @tailwindcss/postcss@4` | Tailwind v4 PostCSS plugin |
| `wasm32-unknown-unknown` | `rustup target add wasm32-unknown-unknown` | Rust WASM target |

## Files to Create

| File | Action |
|------|--------|
| `crates/quilt-ui/Trunk.toml` | Create |
| `crates/quilt-ui/postcss.config.js` | Create |
| `crates/quilt-ui/package.json` | Create |
| `crates/quilt-ui/index.html` | Modify (remove broken script tag, fix stylesheet href) |

## Verification

1. **Trunk serves HTML without errors:**
   ```bash
   cd crates/quilt-ui && trunk serve
   ```
   Should show: `Trunk build completed successfully`

2. **Tailwind processes CSS:**
   ```bash
   npm run tailwind:watch
   ```
   Should create `dist/style.css` with processed Tailwind classes

3. **Browser loads with styles:**
   - Navigate to `http://127.0.0.1:8080`
   - Page should render with dark theme (`bg-base`, `text-text`)

4. **CSS changes reflect in browser:**
   - Edit `style.css`, add a new class like `bg-red-500`
   - Save — Tailwind should reprocess
   - Refresh browser to see changes

5. **WASM loads correctly:**
   - Check browser console for WASM initialization errors
   - The `<div id="app">` should be populated by Leptos

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `wasm32-unknown-unknown` target not installed | Low | High | Document `rustup target add wasm32-unknown-unknown` |
| Two-terminal workflow friction | Medium | Low | Document clearly, provide convenience scripts |
| CSS not reloading in browser | Medium | Medium | Manual browser refresh on CSS change |

## Open Questions

1. **Confirm approach**: Trunk + Tailwind CLI (not cargo-leptos)?
2. **CSS hot-reload**: Is browser auto-refresh on CSS change sufficient, or is full HMR needed?
3. **Backend URL**: `http://127.0.0.1:3541` hardcoded in `bridge.rs` — is this configurable?
