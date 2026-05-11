import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E Test Configuration for Quilt UI
 *
 * This config targets the Leptos/Trunk dev server running on localhost:1420.
 * Run with: npx playwright test
 *
 * NOTE: Before running tests, ensure the dev server is running:
 *   cd crates/quilt-ui && trunk serve --port 1420
 *   OR: cargo run -p quilt-ui (if applicable)
 */
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',

  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
    // Mobile browsers
    {
      name: 'Mobile Chrome',
      use: { ...devices['Pixel 5'] },
    },
    {
      name: 'Mobile Safari',
      use: { ...devices['iPhone 12'] },
    },
  ],

  webServer: {
    command: 'echo "Dev server must be running on port 1420" && exit 1',
    port: 1420,
    reuseExistingServer: !process.env.CI,
  },
});
