# Proposal: Trunk + Tailwind CSS 4 Setup for quilt-ui

## Executive Summary

Set up Trunk as the build tool for `quilt-ui` with Tailwind CSS 4 properly configured. The current `index.html` is broken (references non-existent `./src/main.ts` TypeScript file) and `style.css` has no build pipeline. This change adds the missing configuration files and fixes the broken entry point.

## Intent

Enable local development of the quilt-ui WASM application with:
- Trunk building the Leptos WASM + serving HTML
- Tailwind CSS 4 processing the stylesheet
- A documented two-process dev workflow

## Scope

### In
- Add `Trunk.toml` with Leptos WASM + Tailwind configuration
- Fix `crates/quilt-ui/index.html` to be Trunk-compatible (no TypeScript references)
- Add `postcss.config.js` for Tailwind v4 processing
- Add npm `package.json` with Tailwind CLI scripts
- Document the two-terminal dev workflow

### Out
- Production CSS minification pipeline
- CI/CD integration
- Changes to any Rust source code
- `cargo-leptos` integration (can be added separately)

## Approach

### Files to Create/Modify

| File | Action |
|------|--------|
| `crates/quilt-ui/Trunk.toml` | Create — build config |
| `crates/quilt-ui/postcss.config.js` | Create — Tailwind v4 PostCSS plugin |
| `crates/quilt-ui/package.json` | Create — Tailwind CLI scripts |
| `crates/quilt-ui/index.html` | Fix — remove TS reference, use Trunk-compatible markup |

### Dev Workflow

```bash
# Terminal 1: Watch CSS changes
npm run tailwind:watch  # tailwindcss -i style.css -o dist/style.css --watch

# Terminal 2: Build WASM + serve
trunk serve --open
```

### Configuration Details

**Trunk.toml:**
```toml
[build]
target = "index.html"
dist = "dist"

[watch]
ignore = ["target", "node_modules"]

[build.target]
# Leptos CSR: trunk copies index.html + assets to dist/
```

**postcss.config.js:**
```javascript
module.exports = {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}
```

**index.html (fixed):**
```html
<!DOCTYPE html>
<html lang="en" class="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Quilt</title>
    <link rel="stylesheet" href="style.css">
</head>
<body class="bg-base text-text">
    <div id="app"></div>
</body>
</html>
```

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `wasm32-unknown-unknown` target not installed | Low | High | Document `rustup target add wasm32-unknown-unknown` |
| Two-terminal workflow friction | Medium | Low | Document clearly, provide convenience scripts |
| CSS not reloading in browser | Medium | Medium | Test Tailwind watch mode thoroughly |

## Open Questions

1. **Confirm approach**: Trunk + Tailwind CLI (not cargo-leptos)?
2. **CSS hot-reload**: Is browser auto-refresh on CSS change sufficient, or is full HMR needed?
3. **Backend URL**: `http://127.0.0.1:3541` hardcoded in `bridge.rs` — is this configurable?

## Dependencies

- `trunk` — install via `cargo install trunk`
- `tailwindcss` + `@tailwindcss/postcss` — via npm
- `wasm32-unknown-unknown` Rust target — via `rustup`

## Success Criteria

- [ ] `trunk serve` compiles quilt-ui WASM without errors
- [ ] `tailwindcss --watch` processes `style.css` to `dist/style.css`
- [ ] Browser loads the page with correct Tailwind styles applied
- [ ] `npm run dev` (or equivalent) documented as two-step process
- [ ] No TypeScript or `main.ts` references remain in quilt-ui
