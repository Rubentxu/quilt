# Quilt UI E2E Tests

Playwright-based end-to-end tests for Quilt UI (React frontend).

## Prerequisites

1. **Node.js >= 18** and **npm >= 9**
2. **Playwright browsers** (Chromium, Firefox, WebKit)
3. **Backend server** running on port 3737 (required for most tests)
4. **React dev server** on port 5173 (auto-started by Playwright)

## Quick Start

### 1. Install Dependencies

```bash
cd quilt-ui && npm install
npx playwright install --with-deps
```

### 2. Start Backend

In one terminal, start the Rust backend:

```bash
cargo run -p quilt-server
```

### 3. Run Tests

```bash
# Run all tests
npx playwright test

# Run a specific test file
npx playwright test smoke
npx playwright test outliner
npx playwright test journal
npx playwright test accessibility
npx playwright test visual-regression

# Run in headed mode (see browser)
npx playwright test --headed

# List tests without running
npx playwright test --list
```

## Test Structure

```
tests/e2e/
├── pom/                          # Page Object Model
│   ├── base.page.ts              # Base page object with common methods
│   ├── sidebar.component.ts      # Sidebar navigation component
│   ├── search.page.ts            # Search page object
│   ├── theme-toggle.component.ts # Theme toggle component
│   ├── right-sidebar.component.ts# Right sidebar component
│   └── index.ts                  # Export all POM objects
├── spec/                         # Test specifications
│   ├── navigation.spec.ts        # Sidebar, mobile menu, deep linking, browser nav
│   ├── search.spec.ts            # Search input, results, keyboard nav, empty states
│   ├── theme.spec.ts             # Light/dark theme toggle and persistence
│   ├── right-sidebar.spec.ts     # Right sidebar open/close, tab switching
│   ├── error-handling.spec.ts    # Offline, timeouts, empty states, 404
│   ├── smoke.spec.ts             # App shell, sidebar, theme toggle (React)
│   ├── outliner.spec.ts          # Block create, edit, Enter split, Backspace merge (React)
│   ├── journal.spec.ts           # Journal date header, prev/next day nav (React)
│   ├── inline.spec.ts            # Inline markdown rendering (**bold**, *italic*) (React)
│   ├── markers.spec.ts           # Block bullets, task markers (TODO/DONE) (React)
│   └── page-editing.spec.ts      # Page editing, content persistence (React)
├── graph-view.spec.ts            # P0 graph view tests
└── quilt.spec.ts                 # P0 baseline tests
```

### Test Run Modes

- **No backend**: Basic smoke, DOM structure, theme toggle tests pass
- **With backend**: All tests execute including block editing, journal nav, inline rendering

Environment variable `API_BASE_URL` (default: `http://localhost:3737`) controls which backend the tests target.

## Selectors

Tests use stable `data-testid` attributes.

### App Shell
| Element | Selector |
|---------|----------|
| App container | `[data-testid="app-shell"]` |
| Breadcrumb | `[data-testid="breadcrumb"]` |
| Theme toggle | `[data-testid="theme-toggle"]` |
| Mobile menu button | `[data-testid="mobile-menu-button"]` |

### Navigation
| Element | Selector |
|---------|----------|
| Sidebar | `[data-testid="sidebar"]` |
| Sidebar Journal link | `[data-testid="nav-journal"]` |
| Sidebar Pages link | `[data-testid="nav-pages"]` |
| Sidebar Graph link | `[data-testid="nav-graph"]` |

### Outliner / Blocks
| Element | Selector |
|---------|----------|
| Block row | `.block-row` / `[data-testid^="block-row-"]` |
| Bullet/collapse button | `.block-bullet` |
| Content (read mode) | `.block-content-read` |
| Content (edit mode) | `.block-content[contenteditable]` |

### Journal
| Element | Selector |
|---------|----------|
| Prev day button | `[data-testid="nav-prev-day"]` |
| Next day button | `[data-testid="nav-next-day"]` |

### Search
| Element | Selector |
|---------|----------|
| Search input | `[data-testid="search-input"]` (modal) |
| Sidebar search | `[data-testid="sidebar-search-input"]` |

### Theme
| Element | Selector |
|---------|----------|
| Theme toggle button | `[data-testid="theme-toggle"]` |

## CI Mode

```bash
# Run in CI mode (retries, line reporter, JUnit output)
BASE_URL=http://localhost:5173 npx playwright test --reporter=line,junit
```

## Accessibility Tests

Automated WCAG 2.1 AA scans via `@axe-core/playwright`. Each test page
is scanned with `wcag2a`, `wcag2aa`, `wcag21a`, and `wcag21aa` rule tags.
Tests fail on **serious** or **critical** violations; lower-severity
issues are reported but do not break the build.

```bash
npx playwright test accessibility
```

The suite covers:
- Home page axe scan (no serious/critical violations)
- Main landmark presence
- Keyboard accessibility (Tab focus lands on a visible element)
- Image `alt` text
- Form input labels (for/id, aria-label, placeholder, or ancestor `<label>`)
- Color contrast (WCAG AA)
- Heading hierarchy (no skipped levels, starts at h1)
- `aria-expanded` toggles

## Visual Regression Tests

Screenshot-based regression detection. Baselines are stored alongside
each spec in `tests/e2e/spec/*.spec.ts-snapshots/`.

```bash
# First run — create baselines (commit the snapshot files)
npx playwright test visual-regression --update-snapshots

# Subsequent runs — diff against baselines
npx playwright test visual-regression
```

CI runs visual regression but does **not** fail on first runs (baselines
get created and reviewed in a follow-up PR). Globals live in
`playwright.config.ts` under `expect.toHaveScreenshot`:

```ts
expect: {
  toHaveScreenshot: {
    maxDiffPixels: 100,
    threshold: 0.2,
    animations: 'disabled',
    caret: 'hide',
    scale: 'css',
  },
},
```

The suite covers:
- Home page layout (full page)
- Sidebar layout
- Journal page (today's date)
- Block row default state
- Dark mode
- Mobile viewport (375×667)

## Troubleshooting

### "Dev server must be running on port 5173"

The Playwright config auto-starts the Vite dev server. If it fails:

```bash
cd quilt-ui && npm run dev
```

### Backend not running

Some tests require the backend on port 3737:

```bash
cargo run -p quilt-server
```

Tests that depend on the backend will skip gracefully with `test.skip` if unreachable.

### Browser not found

Reinstall Playwright browsers:

```bash
npx playwright install --with-deps
```

### Tests timeout

Increase timeout in `playwright.config.ts` or run with:

```bash
npx playwright test --timeout=60000
```

## Architecture

These tests are **external E2E tests** that:
- Run against the React/Vite dev server served by Vite
- Use Playwright's cross-browser capabilities
- Target the React UI at `http://localhost:5173`
- Prefer `data-testid` selectors over fragile CSS
- Use REST API calls for test data setup/teardown
