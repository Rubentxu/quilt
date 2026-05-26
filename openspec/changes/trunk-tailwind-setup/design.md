# Design: Trunk + Tailwind CSS 4 Setup (Change trunk-tailwind-setup)

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Development Workflow                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Terminal 1:                     Terminal 2:                     │
│  ┌──────────────────┐           ┌──────────────────────────┐   │
│  │ tailwindcss CLI   │           │ trunk serve              │   │
│  │ -i style.css      │           │ - Builds WASM from Rust  │   │
│  │ -o dist/style.css │           │ - Serves index.html       │   │
│  │ --watch          │           │ - Serves dist/style.css   │   │
│  └────────┬─────────┘           └────────────┬─────────────┘   │
│           │                                    │                │
│           │ copies to dist/                    │ serves on     │
│           ▼                                    ▼                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  dist/style.css  ←── Trunk copies this to dist/          │   │
│  │  index.html      ←── Trunk copies this to dist/          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│                     ┌─────────────────┐                        │
│                     │ Browser :8080    │                        │
│                     │ Loads:           │                        │
│                     │ - index.html     │                        │
│                     │ - dist/style.css │                        │
│                     │ - *.wasm         │                        │
│                     └─────────────────┘                        │
└─────────────────────────────────────────────────────────────────┘
```

**Key insight**: Tailwind CLI runs independently in Terminal 1, outputting processed CSS to `dist/style.css`. Trunk in Terminal 2 builds the WASM and serves files from `dist/`. Trunk does NOT run PostCSS/Tailwind — that's a separate CLI process.

## Files to Create/Modify

### 1. crates/quilt-ui/Trunk.toml (Create)

```toml
[build]
target = "index.html"
dist = "dist"

[watch]
ignore = ["target", "node_modules", "dist"]

[build.target]
# Trunk handles WASM + HTML bundling, copies index.html to dist/
# CSS is pre-built by tailwindcss CLI in Terminal 1

[build.tools]
# Tailwind integration is external (npm run tailwind:watch)
# Trunk only serves static files from dist/
```

### 2. crates/quilt-ui/postcss.config.js (Create)

```javascript
module.exports = {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}
```

### 3. crates/quilt-ui/tailwind.config.js (Create for v4 customization if needed)

```javascript
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./index.html', './src/**/*.{rs,rsx,html}'],
  darkMode: 'class',
}
```

Note: Tailwind CSS 4 uses `@import "tailwindcss"` in CSS and `@theme` for customization. The `tailwind.config.js` is optional for v4 — only needed if you want to customize the theme or content paths.

### 4. crates/quilt-ui/package.json (Create)

```json
{
  "name": "quilt-ui",
  "private": true,
  "scripts": {
    "tailwind:watch": "tailwindcss -i style.css -o dist/style.css --watch",
    "tailwind:build": "tailwindcss -i style.css -o dist/style.css --minify",
    "dev": "npm run tailwind:build && trunk serve --open"
  },
  "devDependencies": {
    "@tailwindcss/postcss": "^4.0.0",
    "tailwindcss": "^4.0.0"
  }
}
```

### 5. crates/quilt-ui/index.html (Modify)

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

Changes from current (line numbers refer to current file):
- **Line 7**: `href="style.css"` → `href="dist/style.css"` (Trunk serves from dist)
- **Line 11**: Remove `<script type="module" src="./src/main.ts"></script>` (broken reference, WASM loads automatically)

## Dev Workflow Details

### Terminal 1: Tailwind watch

```bash
cd crates/quilt-ui
npm run tailwind:watch
```

Exact command: `npx tailwindcss -i style.css -o dist/style.css --watch`

This:
1. Reads `style.css` containing Tailwind v4 syntax (`@import "tailwindcss"`, `@theme`)
2. Processes all classes referenced in your Rust code
3. Outputs browser-ready CSS to `dist/style.css`
4. Watches for changes to `style.css` and rebuilds

### Terminal 2: Trunk dev server

```bash
cd crates/quilt-ui
trunk serve --open
```

This:
1. Builds Leptos WASM from Rust sources
2. Copies `index.html` to `dist/`
3. Serves `dist/` directory at `http://127.0.0.1:8080` (or next available port)

Note: Trunk does NOT rebuild CSS — it just serves what Tailwind CLI produced.

### Production Build

```bash
# Terminal 1: Build minified CSS
cd crates/quilt-ui && npm run tailwind:build

# Terminal 2: Build WASM release
cd crates/quilt-ui && trunk build --release
```

## Files to Create

| File | Action |
|------|--------|
| `crates/quilt-ui/Trunk.toml` | Create |
| `crates/quilt-ui/postcss.config.js` | Create |
| `crates/quilt-ui/tailwind.config.js` | Create (optional for v4, but included for content paths) |
| `crates/quilt-ui/package.json` | Create |
| `crates/quilt-ui/index.html` | Modify (remove broken script tag, fix stylesheet href) |

## Verification Steps

### 1. Verify Tailwind CLI works

```bash
cd crates/quilt-ui
npm install
npm run tailwind:build
```

Expected:
- `dist/style.css` created
- File contains processed Tailwind classes (search for `.bg-base`, `.text-text`)
- No errors in output

### 2. Verify Trunk serves correctly

```bash
cd crates/quilt-ui
trunk serve
```

Expected output:
```
2025-01-01T00:00:00  INFO  trunk — building WASM
2025-01-01T00:00:00  INFO  trunk — build completed successfully
2025-01-01T00:00:00  INFO  trunk — serving at http://127.0.0.1:8080
```

### 3. Verify full dev workflow

**Terminal 1:**
```bash
cd crates/quilt-ui && npm run tailwind:watch
```

**Terminal 2:**
```bash
cd crates/quilt-ui && trunk serve --open
```

1. Browser opens at `http://127.0.0.1:8080`
2. Page renders with dark theme (body has `bg-base text-text` classes)
3. Check `dist/style.css` exists and contains Tailwind output

### 4. Verify CSS changes propagate

1. Edit `crates/quilt-ui/style.css` — add `bg-red-500` to body class
2. Wait for Tailwind CLI to rebuild (Terminal 1 shows "Rebuilding...")
3. Refresh browser (Terminal 2 does NOT auto-reload CSS — manual refresh required)
4. Background should turn red

### 5. Verify WASM loads

```bash
# Check browser console for WASM initialization
# No console errors about missing wasm files
# The #app div should be populated by Leptos hydration
```

## Relationship to Existing Spec

This design implements the configuration specified in `spec.md`:
- Creates `Trunk.toml` with correct `[build]` and `[watch]` sections
- Creates `postcss.config.js` using `@tailwindcss/postcss` plugin
- Creates `package.json` with `tailwind:watch` and `tailwind:build` scripts
- Fixes `index.html` to reference `dist/style.css` and removes broken script tag

The two-terminal workflow is intentional and documented as the simplest reliable approach for Trunk + Tailwind integration without additional tooling like `cargo-leptos`.
