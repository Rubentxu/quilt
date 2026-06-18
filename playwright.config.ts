import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E Test Configuration for Quilt
 *
 * Targets the full stack:
 *   - Backend API server on localhost:3737 (REST + WebSocket)
 *   - React/Vite dev server on localhost:5173 (HMR frontend)
 *
 * Run with: just test-e2e  (or: QUILT_API_KEY=<key> npx playwright test)
 *
 * The webServer block below starts BOTH services before running tests.
 * Tests use http://localhost:3737 for API calls and http://localhost:5173 for UI navigation.
 */
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',

  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },

  // Visual regression defaults — applied to every `toHaveScreenshot` call.
  // Tune per-call via the second arg of `toHaveScreenshot(name, options)`.
  expect: {
    toHaveScreenshot: {
      maxDiffPixels: 100,
      threshold: 0.2, // 20% per-pixel color threshold (anti-aliasing tolerance)
      animations: 'disabled', // disable CSS animations/JS animations for stability
      caret: 'hide', // hide blinking text caret
      scale: 'css', // compare at CSS pixel ratio, not device pixel ratio
    },
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  // Start the full dev stack (backend + frontend) before running tests.
  // dev-react.sh starts: backend (3737), WASM watcher, asset watcher, then Vite (5173).
  // We wait for the frontend (5173) to be ready since that's what tests navigate to.
  webServer: {
    command: './scripts/dev-react.sh',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 180_000, // Full dev stack takes time to compile
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
