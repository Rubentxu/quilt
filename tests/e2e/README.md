# Quilt UI E2E Tests

Playwright-based end-to-end tests for Quilt UI.

## Prerequisites

1. **Node.js >= 18** and **npm >= 9**
2. **Playwright browsers** (Chromium, Firefox, WebKit)
3. **Trunk** (for serving the Leptos UI):
   ```bash
   cargo install trunk
   ```

## Quick Start

### 1. Install Dependencies

```bash
npm install
npx playwright install --with-deps
```

Or via just:
```bash
just e2e-install
```

### 2. Start Dev Server

In one terminal, start the Trunk dev server:

```bash
cd crates/quilt-ui && trunk serve --port 1420
```

Or via just (runs server in background):
```bash
cd crates/quilt-ui && trunk serve --port 1420 &
sleep 3
```

### 3. Run Tests

```bash
# Run all tests
npx playwright test

# Run in headed mode (see browser)
npx playwright test --headed

# List tests without running
npx playwright test --list
```

Or via just:
```bash
just e2e-test        # Run all tests
just e2e-test-headed # Run with browser visible
just e2e-list        # List tests
just e2e-ui          # Open Playwright UI
```

## Test Structure

```
tests/e2e/
├── pom/                    # Page Object Model
│   ├── base.page.ts        # Base page object with common methods
│   ├── sidebar.component.ts # Sidebar navigation component
│   ├── search.page.ts      # Search page object
│   ├── theme-toggle.component.ts # Theme toggle component
│   ├── right-sidebar.component.ts # Right sidebar component
│   └── index.ts           # Export all POM objects
├── spec/                   # Test specifications
│   ├── navigation.spec.ts  # Sidebar, mobile menu, deep linking, browser nav
│   ├── search.spec.ts      # Search input, results, keyboard nav, empty states
│   ├── theme.spec.ts       # Light/dark theme toggle and persistence
│   ├── right-sidebar.spec.ts # Right sidebar open/close, tab switching
│   └── error-handling.spec.ts # Offline, timeouts, empty states, 404
├── quilt.spec.ts           # P0 baseline tests
└── graph-view.spec.ts      # Graph view specific tests
```

### Test Categories

- **Navigation** - Mobile sidebar, active route highlighting, deep linking, back/forward
- **Search** - Search input, full-text search, keyboard navigation, empty states
- **Theme** - Light/dark toggle, persistence across navigation and reload
- **Right Sidebar** - Open/close, tab switching (Properties, Backlinks, Annotations)
- **Error Handling** - Offline handling, API timeouts, empty states, 404 pages
- **Query** - DSL query input, execution, result/error handling
- **Cognitive Dashboard** - Dashboard load, refresh action
- **Pages View** - Page list render, content/empty states
- **Graph View** - Canvas-based force-directed graph visualization

## Selectors

Tests use stable `data-testid` attributes:

### Navigation
| Element | Selector |
|---------|----------|
| Sidebar Journal link | `[data-testid="nav-journal"]` |
| Sidebar Pages link | `[data-testid="nav-pages"]` |
| Sidebar Search link | `[data-testid="nav-search"]` |
| Sidebar Query link | `[data-testid="nav-query"]` |
| Sidebar Graph link | `[data-testid="nav-graph"]` |
| Sidebar Cognitive link | `[data-testid="nav-cognitive"]` |
| Mobile menu button | `[data-testid="mobile-menu-button"]` |

### Search
| Element | Selector |
|---------|----------|
| Search input | `[data-testid="search-input"]` |

### Theme
| Element | Selector |
|---------|----------|
| Theme toggle button | `[data-testid="theme-toggle"]` |

### Right Sidebar
| Element | Selector |
|---------|----------|
| Tab - Properties | `[data-testid="tab-properties"]` |
| Tab - Backlinks | `[data-testid="tab-backlinks"]` |
| Tab - Annotations | `[data-testid="tab-annotations"]` |

### Query
| Element | Selector |
|---------|----------|
| Query input | `[data-testid="query-input"]` |
| Run Query button | `[data-testid="run-query-button"]` |
| Query chip (task todo) | `[data-testid="query-chip-task-todo"]` |
| Query chip (priority a) | `[data-testid="query-chip-priority-a"]` |

### Cognitive Dashboard
| Element | Selector |
|---------|----------|
| Refresh button | `[data-testid="refresh-button"]` |

### Graph View
| Element | Selector |
|---------|----------|
| Graph view container | `[data-testid="graph-view"]` |
| Force graph canvas | `[data-testid="force-graph"]` |
| Graph canvas | `[data-testid="graph-canvas"]` |
| Graph controls | `[data-testid="graph-controls"]` |
| Graph legend | `[data-testid="graph-legend"]` |
| Zoom in button | `[data-testid="zoom-in"]` |
| Zoom reset button | `[data-testid="zoom-reset"]` |
| Zoom out button | `[data-testid="zoom-out"]` |
| Graph filter (pages) | `[data-testid="graph-filter-pages"]` |
| Graph filter (journals) | `[data-testid="graph-filter-journals"]` |
| Graph error state | `[data-testid="graph-error"]` |
| Graph empty state | `[data-testid="graph-empty"]` |
| Graph retry button | `[data-testid="graph-retry-button"]` |
| Graph error message | `[data-testid="graph-error-message"]` |

## CI Mode

```bash
# Run in CI mode (retries, line reporter, JUnit output)
just e2e-test-ci
```

## Troubleshooting

### "Dev server must be running on port 1420"

Start the Trunk dev server first:
```bash
cd crates/quilt-ui && trunk serve --port 1420
```

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
- Run against the compiled WASM UI served by Trunk
- Use Playwright's cross-browser capabilities
- Target the Leptos SPA at `http://localhost:1420`
- Avoid fragile CSS selectors in favor of `data-testid`
