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
└── quilt.spec.ts    # All P0 baseline tests
```

### Test Categories

- **Sidebar Navigation** - Route changes via sidebar links
- **Search** - Search input, Enter key, results/empty states
- **Query** - DSL query input, execution, result/error handling
- **Cognitive Dashboard** - Dashboard load, refresh action
- **Pages View** - Page list render, content/empty states

## Selectors

Tests use stable `data-testid` attributes:

| Element | Selector |
|---------|----------|
| Sidebar Journal link | `[data-testid="nav-journal"]` |
| Sidebar Pages link | `[data-testid="nav-pages"]` |
| Sidebar Search link | `[data-testid="nav-search"]` |
| Sidebar Query link | `[data-testid="nav-query"]` |
| Sidebar Cognitive link | `[data-testid="nav-cognitive"]` |
| Search input | `[data-testid="search-input"]` |
| Query input | `[data-testid="query-input"]` |
| Run Query button | `[data-testid="run-query-button"]` |
| Cognitive refresh | `[data-testid="refresh-button"]` |
| Query chip (task todo) | `[data-testid="query-chip-task-todo"]` |
| Query chip (priority a) | `[data-testid="query-chip-priority-a"]` |

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
