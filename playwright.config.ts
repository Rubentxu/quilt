import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E Test Configuration for Quilt UI
 *
 * This config targets the React/Vite dev server running on localhost:5173.
 * Run with: npx playwright test
 *
 * NOTE: Before running tests, ensure the backend server is running:
 *   cargo run -p quilt-server
 *
 * And the frontend dev server is running:
 *   cd quilt-ui && npm run dev
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',

  use: {
    baseURL: process.env.BASE_URL || 'http://localhost:5173',
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

  webServer: {
    command: 'cd quilt-ui && npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },
});
