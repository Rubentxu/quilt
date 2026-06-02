# Performance Budget — Quilt UI

> Source of truth for the size and timing budgets enforced on the
> React frontend. The numbers below are measured against
> `quilt-ui/dist/` after `npm run build`. Bundle guardrails are
> codified as Vitest tests in `quilt-ui/src/__tests__/bundle.test.ts`.

## Why a budget?

The React app has grown quickly: a single-block editor became a
full outliner with a knowledge graph, search, properties, and
collaborative editing. Without a budget, every dependency creep or
new feature pushes the critical-path bundle higher and the WASM
binary stays frozen. The budget turns that into an explicit
trade-off: if a feature pushes us over, we either code-split harder
or defer the feature.

## Bundle size

Measured on a fresh `npm run build` (Vite 6.4.3, esbuild minify,
es2020 target, no terser console stripping in dev — production
drops `console.log` calls).

| Asset | Raw | Gzipped | Loaded on |
| --- | ---: | ---: | --- |
| `index.html` | 0.94 kB | 0.44 kB | first paint |
| `index-*.css` | 12.90 kB | 3.90 kB | first paint |
| `index-*.js` (entry) | 47.06 kB | 13.70 kB | first paint |
| `react-vendor-*.js` | 193.81 kB | 60.38 kB | first paint |
| `router-vendor-*.js` | 82.48 kB | 26.94 kB | first paint |
| `icons-vendor-*.js` | 25.52 kB | 5.20 kB | first paint |
| `toast-vendor-*.js` | 9.75 kB | 3.65 kB | first paint |
| `vendor-misc-*.js` | 4.54 kB | 2.21 kB | first paint |
| `HomePage-*.js` | 0.05 kB | 0.07 kB | first paint (entry route) |
| **Initial JS + CSS gzipped** | — | **~117 kB** | first paint |
| `quilt_core_bg-*.wasm` | 1,757.56 kB | 461.98 kB | lazy on first WASM use |

### Budgets enforced in CI

The numbers below are hard limits. Crossing them is a release
blocker; bump the budget deliberately and document it in
`CHANGELOG.md`.

| Budget | Value | Why |
| --- | ---: | --- |
| `index-*.js` (entry) | < 50 kB gzipped | Was 132.85 kB before code splitting |
| `react-vendor-*.js` | < 80 kB gzipped | React + react-dom + scheduler only |
| `router-vendor-*.js` | < 40 kB gzipped | TanStack router only |
| `quilt_core_bg-*.wasm` | < 600 kB gzipped | Engine is lazy; size cap keeps it from creeping |
| **Initial JS gzipped** | < 150 kB | Roughly the pre-split baseline minus 10% |
| **Initial transfer (JS + CSS + HTML)** | < 200 kB gzipped | Target for `4G fast` (1.6 Mbps) TTI in < 1s |
| **WASM + initial** | < 800 kB gzipped | Total transfer before user can interact with a page |

### Lazy chunks (loaded on demand)

| Chunk | Raw | Gzipped | Loaded by |
| --- | ---: | ---: | --- |
| `PageView-*.js` | 52.20 kB | 14.42 kB | visiting `/page/:name` or `/journal/:date` |
| `PageViewPage-*.js` | 0.90 kB | 0.55 kB | route shell for `/page/:name` |
| `JournalPage-*.js` | 2.36 kB | 1.09 kB | route shell for `/journal/:date` |
| `SettingsPage-*.js` | 5.92 kB | 2.04 kB | visiting `/settings` |
| `AllPagesPage-*.js` | 6.37 kB | 1.97 kB | visiting `/pages` |
| `GraphViewPage-*.js` | 7.35 kB | 2.86 kB | visiting `/graph` |
| `virtuoso-vendor-*.js` | 55.05 kB | 19.07 kB | when `PageView` mounts |
| `dnd-vendor-*.js` | 43.68 kB | 14.64 kB | when `PageView` mounts |
| `SlashCommandMenu-*.js` | 7.52 kB | 2.43 kB | user types `/` in a block |
| `BlockPropertiesPanel-*.js` | 4.32 kB | 1.48 kB | user opens the properties panel |
| `HoverPreview-*.js` | 2.50 kB | 1.14 kB | user hovers a page reference |
| `PageAutocomplete-*.js` | 1.42 kB | 0.78 kB | user types `[[` in a block |
| `SearchModal-*.js` | 3.74 kB | 1.59 kB | user opens the command palette (Ctrl+K) |

## Code splitting

| Surface | Strategy |
| --- | --- |
| Route components | `lazyRouteComponent` from `@tanstack/react-router` (one chunk per page) |
| Graph view | Lazy route (canvas + force simulation is ~7 kB) |
| Settings | Lazy route (no editor deps needed) |
| All-pages list | Lazy route |
| Search modal | `React.lazy` + `<Suspense>`; mounted only when search opens |
| Slash command menu | `React.lazy` + `<Suspense>`; only fetched when the user types `/` |
| Page autocomplete | `React.lazy` + `<Suspense>`; only fetched when the user types `[[` |
| Block properties panel | `React.lazy` + `<Suspense>`; only fetched when a block opens properties |
| Hover preview | `React.lazy` + `<Suspense>`; only fetched on link hover |
| `react-virtuoso` | Vendor chunk; only fetched with `PageView` |
| `@dnd-kit/*` | Vendor chunk; only fetched with `PageView` |
| `lucide-react` | Vendor chunk (tree-shakable, but a single chunk caches better) |
| WASM | Lazy `ensureWasmLoaded()`; first call wins, the promise is cached |

## Preloading

The router is configured with `defaultPreload: 'intent'`, so the
chunk for a route starts downloading the moment the user hovers or
focuses a `Link` to that route. With a 30-second `preloadStaleTime`,
the preloaded chunk stays warm even if the user hesitates.

```ts
// src/router.tsx
export const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
  defaultPreloadStaleTime: 30_000,
})
```

No additional hover handlers are needed — TanStack Router wires the
preload to every `Link` in the app.

## WASM loading

WASM is loaded **on first use**, not on app mount. The `WasmProvider`
no longer auto-fetches the engine in a `useEffect`; instead, the
provider exposes `ensureWasmLoaded()` and `getWasmLoadState()`.

Consumers (`PageView`, `InlineContent`) call `ensureWasmLoaded()`
before invoking any WASM function. The promise is cached at module
scope, so concurrent calls share one fetch.

```ts
// In a component that needs WASM
if (!wasmLoaded) {
  await ensureWasmLoaded()
}
wasmLoadPage(pageName, fetchedBlocks)
```

This keeps routes that never touch the engine (Settings, AllPages,
Graph, Home) from paying the ~462 kB gzipped WASM transfer on
first paint.

## Timing budgets

| Metric | Target | Measurement |
| --- | ---: | --- |
| First Contentful Paint (FCP) | < 1.0 s | lighthouse / web-vitals |
| Largest Contentful Paint (LCP) | < 2.5 s | lighthouse / web-vitals |
| Time to Interactive (TTI) | < 2.0 s | lighthouse / manual |
| Total Blocking Time (TBT) | < 200 ms | lighthouse |
| Cumulative Layout Shift (CLS) | < 0.1 | lighthouse |
| WASM load (cold) | < 1.5 s on broadband | `performance.now()` around `loadWasm()` |
| Route chunk fetch (warm) | < 100 ms | TanStack preload timing |

These are aspirational targets — none of them are wired up to a CI
gate yet. Hook them up via `web-vitals` and a Lighthouse CI run
once the dev workflow stabilises.

## Optimization techniques

1. **Route-level code splitting** via `lazyRouteComponent` so each
   page is its own chunk.
2. **Component-level code splitting** for overlays
   (`SearchModal`, `SlashCommandMenu`, `HoverPreview`, etc.) that
   are mounted on user action only.
3. **Vendor chunking** via Vite `manualChunks`: `react-vendor`,
   `router-vendor`, `dnd-vendor`, `virtuoso-vendor`, `icons-vendor`,
   `toast-vendor`, `vendor-misc`. Each can be cached independently.
4. **WASM lazy loading** via `ensureWasmLoaded()` — first use wins,
   the promise is cached.
5. **Preload on intent** via TanStack Router's
   `defaultPreload: 'intent'`. Hover/focus a `Link` and the chunk
   starts downloading.
6. **es2020 target** so we don't ship syntax polyfills for evergreen
   browsers.
7. **CSS minification** via esbuild (`cssMinify: true`).
8. **Tree-shakeable imports** for `lucide-react` (named imports,
   not `import * as`).
9. **`usePerformance` hook** at `src/shared/hooks/usePerformance.ts`
   to surface any component that takes longer than 16 ms (one
   frame) to mount/unmount.

## How to verify

```bash
cd quilt-ui
npm run build
ls -lh dist/assets/
du -sh dist/

# Run the bundle budget tests
npx vitest run src/__tests__/bundle.test.ts
```

The Vitest suite is the contract. If a change makes a chunk fat
enough to bust a budget, the test fails with a clear message
pointing to the chunk and the limit. The intent is to catch the
regression in PR review, not to fight it in production.
