# Sidebar Feature

The Quilt sidebar — quick navigation, recent pages, templates, and
favourites. Lives under `quilt-ui/src/features/sidebar/`.

## Component Structure

```
features/sidebar/
├── Sidebar.tsx                 # Top-level sidebar. Composes the section components.
├── storage-keys.ts             # Central registry of localStorage keys owned by the sidebar.
├── sections/
│   ├── GroupHeader.tsx         # Section label ("Páginas", "Plantillas", "Recientes", …).
│   ├── SidebarItem.tsx         # Single row in a section list.
│   ├── SidebarSkeleton.tsx     # Loading placeholder (shared by Pages and Templates).
│   ├── TemplateSection.tsx     # "Plantillas" — lists templates, click → create + navigate.
│   ├── RecentsSection.tsx      # "Recientes" — last 5 visited pages, localStorage-backed.
│   └── index.ts                # Barrel re-export.
└── __tests__/                  # Vitest suites colocated with the source.
    ├── GroupHeader.test.tsx
    ├── SidebarItem.test.tsx
    ├── SidebarSkeleton.test.tsx
    ├── TemplateSection.test.tsx
    ├── RecentsSection.test.tsx
    └── storage-keys.test.ts
```

The split is intentional: the top-level `Sidebar.tsx` composes
section components rather than carrying every concern. Each section is
small, focused, and individually testable.

## STORAGE_KEYS Convention

All `localStorage` keys owned by the sidebar live in
[`storage-keys.ts`](./storage-keys.ts):

```ts
export const STORAGE_KEYS = {
  /** Page names the user has starred (DESIGN.md §4.1). */
  FAVORITES: 'quilt-favorites',
  /** Most recently visited pages, capped at 5, newest first. */
  RECENTS: 'quilt-recents',
} as const
```

Rules:

1. **Namespace prefix** — every key starts with `quilt-`. This is the
   project's informal namespace for keys read by the frontend, so a
   `localStorage.clear()` from another tool won't wipe our data and we
   can grep the codebase to find all sidebar persistence in one shot.
2. **All keys declared here, not inline** — no
   `localStorage.setItem('quilt-recents', ...)` scattered through
   components. New keys go into `STORAGE_KEYS` and consumers import the
   constant. This is enforced by the test in
   `__tests__/storage-keys.test.ts`.
3. **`as const`** — the keys are typed as a literal union
   (`'quilt-favorites' | 'quilt-recents'`) so the compiler catches typos
   in callers.

## localStorage Migration Path (V1 → V2)

The current implementation is V1: recents and favourites live entirely
in the user's browser. V2 (deferred to `quilt-fase2-server-favorites`)
moves the source of truth to the server so:

- A user signing in on a new device sees their favourites and recents
  immediately.
- Multiple devices stay in sync without needing a browser profile.

The V1 → V2 migration is a **read-prefer-server, write-eagerly** flow:

```
                ┌──────────────────────────────┐
                │  Component mount (V1 or V2)  │
                └──────────────┬───────────────┘
                               │
                ┌──────────────▼───────────────┐
                │  Is the server-side feature  │
                │  available? (capability      │
                │  probe — feature flag)       │
                └──────────────┬───────────────┘
                               │
                ┌───── no ─────▼────── yes ────┐
                │                              │
       ┌────────▼─────────┐        ┌───────────▼──────────┐
       │  V1: read from   │        │  V2: read from the   │
       │  localStorage    │        │  server (authorita-   │
       │  (authoritative) │        │  tive), then hydrate  │
       │                  │        │  localStorage as a    │
       │                  │        │  read-through cache   │
       └──────────────────┘        └──────────────────────┘
```

### Steps

1. **Add a capability probe** — the server exposes
   `GET /api/v1/settings/capabilities` returning the set of optional
   features it supports. The frontend reads it once on app start and
   caches it in a `SidebarCapabilities` context.

2. **Read-prefer-server, fall-back-to-local** — when the probe reports
   `server-favorites: true` (V2), `RecentsSection` and the favourites
   list read from the server. On the very first read (cold cache), the
   client still falls back to `localStorage` so a user with no network
   sees *something*.

3. **Eagerly mirror writes to both** — when the user adds a favourite
   or visits a page, the client optimistically updates the in-memory
   list, writes to `localStorage` (keeps the V1 path warm), and fires
   an async request to the server. The server is authoritative; a
   failed write surfaces a toast and reverts the optimistic update.

4. **One-time backfill on first V2 mount** — when the client detects
   the V2 capability for the first time, it reads `localStorage` and
   posts the contents to `POST /api/v1/users/me/favorites/import` and
   `…/recents/import` (idempotent: deduplicated server-side). After
   the import returns 200, the localStorage entries are kept as a
   read-through cache and never re-imported.

5. **Schema versioning on disk** — every `localStorage` payload carries
   a `version` field. The V1 shape is the current
   `Array<{name, url, visitedAt: number}>`. V2 reads should ignore
   unknown fields and migrate forward when the shape changes. The
   current `isValidRecent` type guard in `RecentsSection.tsx` is the
   hook point for this — it will become a versioned migrator.

### What changes in the components

- `Sidebar.tsx` adds a `useSidebarCapabilities()` hook and passes the
  result down. The section components stay presentational and
  agnostic about where the data lives.
- `RecentsSection.tsx` gains a `useRecentsSource()` hook that returns
  `{items, append, remove}` and hides the server-vs-localStorage
  decision behind that hook. Existing tests in
  `__tests__/RecentsSection.test.tsx` will need a new mock seam for
  the V2 source.
- `storage-keys.ts` is unchanged — the keys stay the same, only the
  writers and readers change. The V1 path remains a safety net.

### Why not just delete V1

- V1 works offline, V2 may not always.
- V1 is fast — `localStorage` reads are synchronous at the page level.
- V1 is robust — no server roundtrip on every navigation.

The V1 layer is kept as a cache (read-through, write-through) so the
server can fail without losing the user's recents, and so a user with
no network still gets a useful sidebar.
