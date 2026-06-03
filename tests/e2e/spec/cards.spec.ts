/**
 * Cards E2E Tests (updated for ADR-0007)
 *
 * Tests for card rendering via the data-driven CardRenderer.
 * Pre-ADR-0007, this spec tested the hardcoded ReferenceCard and
 * ContentCard components; those were replaced by the single
 * CardRenderer which reads `template::` from the block and
 * resolves to a card-shape from the template page.
 *
 * Run with: QUILT_API_KEY=<key> npx playwright test cards
 */

import { test, expect, type Page } from '@playwright/test'
import { getAuthHeaders } from '../auth-state'

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737'
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173'

// ── Helpers ─────────────────────────────────────────────────────

async function createBlock(
  page: Page,
  pageName: string,
  content: string,
  properties?: Record<string, unknown>,
): Promise<string> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content, properties },
    headers,
  })
  if (!resp.ok()) {
    const body = await resp.text()
    throw new Error(`createBlock failed with ${resp.status()}: ${body}`)
  }
  const json = (await resp.json()) as { id: string }
  return json.id
}

async function goToPage(page: Page, pageName: string): Promise<void> {
  await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`)
  const main = page.locator('main')
  await expect(main).toBeVisible({ timeout: 15000 })
}

function suffix(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
}

test.describe('Card-wrapped blocks (ADR-0007)', () => {
  // ── type:: reference → legacy fallback ─────────────────────

  test('type:: reference block renders a card via legacy fallback', async ({ page }) => {
    const pageName = `cards-ref-${suffix()}`

    await createBlock(page, pageName, 'DDA Huella v1.0.0', {
      'type': 'reference',
      'dda-relacionada': 'DDA Huella',
    })

    await goToPage(page, pageName)
    const card = page.locator('[data-testid="card-renderer"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
    await expect(page.getByText('DDA Huella v1.0.0').first()).toBeVisible()
    await expect(page.getByText('DDA Huella').first()).toBeVisible()
  })

  // ── type:: documentacion → legacy fallback ─────────────────

  test('type:: documentacion block renders a collapsible card', async ({ page }) => {
    const pageName = `cards-doc-${suffix()}`

    await createBlock(page, pageName, 'Documentación Pipelines', {
      'type': 'documentacion',
    })

    await goToPage(page, pageName)
    const card = page.locator('[data-testid="card-renderer"][data-shape="content"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
    await expect(page.getByText('Documentación Pipelines').first()).toBeVisible()
  })

  // ── "+ Reference" quick-add button ──────────────────────────

  test('+ Reference quick-add button creates a block with template:: reference', async ({ page }) => {
    const pageName = `cards-qref-${suffix()}`

    await goToPage(page, pageName)
    const main = page.locator('main')
    await expect(main).toBeVisible({ timeout: 15000 })

    const refButton = page.getByRole('button', { name: 'Add reference' })
    if (await refButton.isVisible()) {
      await refButton.click()
      await expect(page.getByText('Reference').first()).toBeVisible({ timeout: 5000 })
    }
  })

  // ── "+ Documentation" quick-add button ──────────────────────

  test('+ Documentation quick-add button creates a block with template:: documentation', async ({ page }) => {
    const pageName = `cards-qdoc-${suffix()}`

    await goToPage(page, pageName)
    const main = page.locator('main')
    await expect(main).toBeVisible({ timeout: 15000 })

    const docButton = page.getByRole('button', { name: 'Add documentation' })
    if (await docButton.isVisible()) {
      await docButton.click()
      await expect(page.getByText('Documentation').first()).toBeVisible({ timeout: 5000 })
    }
  })
})
