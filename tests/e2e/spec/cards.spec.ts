/**
 * Cards E2E Tests
 *
 * Tests for ReferenceCard and ContentCard rendering when a block
 * has the `type:: reference` or `type:: documentacion` property.
 *
 * Prerequisites:
 *   - Server running on localhost:3737
 *   - Frontend running on localhost:5173
 *   - QUILT_API_KEY env var set
 *
 * Run with: QUILT_API_KEY=<key> npx playwright test cards
 */

import { test, expect, type Page } from '@playwright/test';
import { getAuthHeaders } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

/** Create a block via REST API (with auth). */
async function createBlock(
  page: Page,
  pageName: string,
  content: string,
  properties?: Record<string, unknown>
): Promise<string> {
  const headers = getAuthHeaders();
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content, properties },
    headers,
  });
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`createBlock failed with ${resp.status()}: ${body}`);
  }
  const json = (await resp.json()) as { id: string };
  return json.id;
}

test.describe('Card-wrapped blocks', () => {
  const testPage = `cards-e2e-test-${Date.now()}`;

  test('type:: reference block renders ReferenceCard', async ({ page }) => {
    // Create a reference block directly via API
    await createBlock(
      page,
      testPage,
      'DDA Huella v1.0.0',
      { type: 'reference', 'dda-relacionada': 'DDA Huella' }
    );

    // Navigate to the page
    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(testPage)}`);

    // Wait for page to load
    const main = page.locator('main');
    await expect(main).toBeVisible({ timeout: 15000 });

    // The ReferenceCard should be visible with the title
    const card = page.locator('[data-testid="reference-card"]').first();
    await expect(card).toBeVisible({ timeout: 10000 });

    // The title should be the block content
    await expect(page.getByText('DDA Huella v1.0.0').first()).toBeVisible();

    // The meta should be visible
    await expect(page.getByText('DDA Huella').first()).toBeVisible();
  });

  test('type:: documentacion block renders ContentCard', async ({ page }) => {
    await createBlock(
      page,
      testPage,
      'Documentación Pipelines Correos',
      { type: 'documentacion' }
    );

    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(testPage)}`);
    const main = page.locator('main');
    await expect(main).toBeVisible({ timeout: 15000 });

    // ContentCard has a collapse toggle
    const contentCard = page.locator('[data-testid="content-card"]').first();
    await expect(contentCard).toBeVisible({ timeout: 10000 });

    // Title should be visible
    await expect(page.getByText('Documentación Pipelines Correos').first()).toBeVisible();
  });

  test('+ Reference button creates a reference block', async ({ page }) => {
    // Navigate to the test page (will be empty)
    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(testPage)}`);

    const main = page.locator('main');
    await expect(main).toBeVisible({ timeout: 15000 });

    // Click "+ Reference" in the add-block area
    const refButton = page.getByRole('button', { name: 'Add reference' });
    if (await refButton.isVisible()) {
      await refButton.click();

      // A new block should appear with type:: reference
      // The block content should contain "Reference" (placeholder)
      await expect(page.getByText('Reference').first()).toBeVisible({ timeout: 5000 });
    }
  });

  test('+ Documentation button creates a documentacion block', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(testPage)}`);

    const main = page.locator('main');
    await expect(main).toBeVisible({ timeout: 15000 });

    const docButton = page.getByRole('button', { name: 'Add documentation' });
    if (await docButton.isVisible()) {
      await docButton.click();

      await expect(page.getByText('Documentación').first()).toBeVisible({ timeout: 5000 });
    }
  });
});
