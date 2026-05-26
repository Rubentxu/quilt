# Exploration: Trunk + Tailwind CSS 4 Setup for quilt-ui

## Current State

### quilt-ui Architecture

**`crates/quilt-ui/Cargo.toml`** — Current setup:
- Leptos 0.8 with `csr` feature (client-side rendering)
- `crate-type = ["cdylib", "rlib"]` — compiles to WASM
- Dependencies: `leptos`, `leptos_router`, `leptos_meta`, `wasm-bindgen`, `gloo`, `console_log`, `console_error_panic_hook`
- Description claims "Tailwind CSS 4" but build pipeline is missing

**`crates/quilt-ui/src/lib.rs`** — WASM entry:
```rust
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();
    leptos::mount::mount_to_body(|| {
        leptos::view! { <app::App /> }
    });
}
```
Standard Leptos CSR WASM mount pattern.

**`crates/quilt-ui/index.html`** — **PROBLEM: Wrong entry point**:
```html
<script type="module" src="./src/main.ts"></script>
```
This references TypeScript (`main.ts`) which is completely wrong for Leptos WASM. A Trunk-compatible index.html should either:
- Reference the compiled WASM directly (for manual builds)
- Use Trunk's asset pipeline (for automatic WASM + JS generation)

**`crates/quilt-ui/style.css`** — Tailwind CSS 4 syntax present:
```css
@import "tailwindcss";
@theme {
    --color-base: #002b36;
    --color-surface: #073642;
    /* ... custom theme tokens */
}
```
This is correct Tailwind v4 CSS-first syntax. However, there's no build pipeline to process it.

### What's Missing
| File | Status |
|------|--------|
| `Trunk.toml` | ❌ Missing |
| `tailwind.config.js` | ❌ Not needed for v4 CSS-first |
| `postcss.config.js` | ❌ Missing |
| Tailwind build pipeline | ❌ Missing |
| Correct `index.html` | ❌ Wrong (TS references) |

---

## Affected Areas

- `crates/quilt-ui/Cargo.toml` — may need `console_log` feature or additional deps
- `crates/quilt-ui/index.html` — **must be rewritten** for Trunk
- `crates/quilt-ui/style.css` — needs Tailwind CLI build step
- `crates/quilt-ui/src/lib.rs` — already correct WASM entry
- Workspace `Cargo.toml` — no changes likely needed
- No `.fingerprint/` or `target/` changes needed

---

## Approaches

### Approach 1: Trunk + Tailwind CLI (Minimal)

**Trunk** handles WASM compilation + asset pipeline. **Tailwind CLI** processes CSS separately.

**Trunk.toml:**
```toml
[build]
target = "index.html"
dist = "dist"

[watch]
ignore = ["target"]

[build.target]
# For Leptos CSR, trunk copies index.html + assets to dist/
```

**index.html (Trunk-compatible):**
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
    <!-- Trunk injects WASM here -->
</body>
</html>
```

**Build workflow:**
```bash
# Terminal 1: Watch CSS
tailwindcss -i style.css -o dist/style.css --watch

# Terminal 2: Build WASM + assets
trunk serve --open
```

**Pros:** Minimal changes, standard Trunk usage, clear separation
**Cons:** Two terminals for dev, manual CSS rebuilds

### Approach 2: cargo-leptos (Integrated)

`cargo-leptos` wraps Trunk + provides CSS hot-reload, SCSS, parallel builds.

**Add to Cargo.toml:**
```toml
[package.metadata.leptos]
# cargo-leptos config
```

**Requires:**
- `cargo install cargo-leptos`
- `package.metadata.leptos` in Cargo.toml
- Tailwind CLI or PostCSS for CSS

**Pros:** Single command, integrated watch mode, better DX
**Cons:** Extra dependency, more complex config, cargo-leptos wraps Trunk anyway

### Approach 3: Trunk + PostCSS (Standard Web Stack)

Use PostCSS with Tailwind v4 plugin for standard web development.

**postcss.config.js:**
```javascript
module.exports = {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}
```

**index.html:** Same as Approach 1
**Trunk.toml:** Same as Approach 1

**Build:**
```bash
# PostCSS CLI or integrate with Trunk's asset pipeline
```

**Pros:** Standard web toolchain, familiar for web devs
**Cons:** More dependencies, PostCSS may not integrate directly with Trunk's asset watching

---

## Tailwind CSS 4 Specifics

**Tailwind v4 key differences from v3:**
- CSS-first configuration: no `tailwind.config.js` needed (can still use one)
- `@import "tailwindcss"` replaces `@tailwind base/components/utilities`
- `@theme {}` block for custom tokens (already used in existing style.css)
- Rust engine (`tailwindcss-oxide`) for performance
- Can use `@tailwindcss/postcss` or `@tailwindcss/cli`

**For Trunk integration:**
Trunk handles WASM + HTML. CSS processing is separate. Options:
1. Tailwind CLI: `tailwindcss -i style.css -o dist/style.css`
2. PostCSS: `postcss style.css -o dist/style.css`
3. Build script hook in Trunk.toml

---

## cargo-leptos vs trunk vs both

| Tool | Role | Needed? |
|------|------|---------|
| `trunk` | Build tool for WASM + web assets | **Yes** |
| `cargo-leptos` | Wrapper with extra features (hot-reload, SCSS) | Optional |
| `tailwindcss cli` | CSS processing | **Yes** |

**Decision:** Use **Trunk + Tailwind CLI** (Approach 1). Simpler, fewer dependencies, standard pattern.

If CSS hot-reload is critical, consider `cargo-leptos` later.

---

## CLI vs WASM-only

**Current state:** quilt-ui is a WASM library only. No CLI entry point.

- `lib.rs` exports `main()` for WASM browser execution
- No binary crate (`[[bin]]`) in quilt-ui
- The `index.html` is the browser entry point
- Backend communication via HTTP (bridge.rs → `http://127.0.0.1:3541/api`)

**Question for spec phase:** Is there a CLI host planned? If so:
- CLI would serve the WASM + static files
- Could use `tiny_http` or `axum` for static file serving
- Or rely on separate server (quilt-mcp? quilt-bin?)

---

## Risks

1. **Broken index.html** — Current `main.ts` reference is completely wrong. Must be fixed before anything works.
2. **CSS not processed** — `style.css` with Tailwind v4 directives won't work without build step.
3. **No WASM target** — Haven't verified `wasm32-unknown-unknown` target is installed.
4. **Duplicate styling** — The `style.css` is imported via `<Stylesheet href="/style.css" />` in `app.rs` (Leptos) AND via `<link rel="stylesheet">` in index.html. Need to consolidate.
5. **Watch mode complexity** — Two-terminal dev workflow is error-prone.

---

## Entropy Analysis (Connascence Landscape)

**Method**: Heuristic (CogniCode graph available but focused on architecture)

| Component A | Component B | Connascence Type | I(bits) | Severity |
|------------|-------------|------------------|---------|----------|
| `style.css` | Tailwind v4 | Meaning | 1.5 | ⚠️ Medium |
| `index.html` | WASM entry | Position | 2.0 | ⚠️ Medium |
| `lib.rs` | `app.rs` | Name | 0.32 | ✅ OK |
| `bridge.rs` | Backend API | Meaning | 0.82 | ⚠️ Low |
| `index.html` | `style.css` | Position | 0.58 | ⚠️ Low |

**Critical Pairs (I > 3.0 bits)**: None
**Hidden Connascence (Meaning/Timing)**: 
- `BASE_URL: "http://127.0.0.1:3541"` hardcoded in bridge.rs — timing assumption that backend runs on this port
- `index.html` references `./src/main.ts` which doesn't exist — meaning: broken entry point

**Coupling Score**: H_external = Low (quilt-ui is self-contained, only depends on Leptos + Tailwind)
**Estimation Method**: Heuristic
**Confidence**: estimated

---

## Open Questions Before Spec

1. **Is cargo-leptos acceptable as a dependency?** Or strictly trunk + tailwind cli?
2. **CSS hot-reload requirement?** Affects approach choice.
3. **Is there a CLI host planned?** If quilt-ui needs a host binary, we need to plan for that.
4. **Backend URL hardcoded** (`127.0.0.1:3541`) — is this configurable or永远的?
5. **Current style.css via Leptos `<Stylesheet>` vs `<link>` in HTML** — consolidate or keep both?
6. **Tailwind v4 `@tailwindcss/postcss` vs `@tailwindcss/cli`** — which for Rust ecosystem?

---

## Recommendation

**Approach 1 (Trunk + Tailwind CLI)** with these steps:

1. Fix `index.html` to be Trunk-compatible (no TS references)
2. Add `Trunk.toml` for build configuration
3. Add Tailwind CLI build step (or PostCSS)
4. Create a simple `Makefile` or script for the two-terminal dev workflow
5. Consider adding `package.metadata.leptos` later if hot-reload is critical

**Effort**: Medium — primarily configuration files and fixing index.html.

---

## Ready for Proposal

**Yes** — with these clarifications needed:
- Confirm: trunk + tailwind cli (not cargo-leptos)?
- Confirm: CSS hot-reload requirement?
- Decision: CLI host for WASM (yes/no)?
