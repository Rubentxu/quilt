/**
 * Template Card E2E Tests (ADR-0007)
 *
 * End-to-end validation of the template-driven card system.
 *
 * Because the E2E config runs with `fullyParallel: true`, every
 * test uses a unique random suffix to avoid page-name collisions
 * (the server has a UNIQUE constraint on page name).
 *
 * Run with: QUILT_API_KEY=<key> npx playwright test template-card
 */

import { test, expect, type Page } from '@playwright/test'
import { getAuthHeaders } from '../auth-state'

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737'
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173'

// ── Helpers ─────────────────────────────────────────────────────

async function createPage(page: Page, name: string): Promise<void> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/pages`, {
    data: { name },
    headers,
  })
  if (!resp.ok()) {
    const body = await resp.text()
    throw new Error(`createPage(${name}) failed with ${resp.status()}: ${body}`)
  }
}

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

/** Short unique suffix shared by all artifacts in one test case. */
function suffix(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
}

// ── Tests ───────────────────────────────────────────────────────

test.describe('Template-driven block cards (ADR-0007)', () => {
  // ── template/reference → reference card ────────────────────

  test('template/reference block renders as a reference card', async ({ page }) => {
    const s = suffix()
    const templatePage = `template/ref-${s}`
    const dataPage = `cards-ref-${s}`

    await createPage(page, templatePage)
    await createBlock(page, templatePage, '', {
      'card-shape': 'reference',
      'icon': '🔗',
    })

    await createBlock(page, dataPage, 'DDA Huella v2', {
      'template': `ref-${s}`,
      'dda-relacionada': 'DDA Huella v2',
      'author': 'claude',
    })

    await goToPage(page, dataPage)
    const card = page.locator('[data-testid="card-renderer"][data-shape="reference"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
    await expect(card).toHaveAttribute('data-template', `ref-${s}`)
    await expect(page.getByText('DDA Huella v2').first()).toBeVisible()
    // Metas rendered inside the card
    await expect(page.getByText('dda-relacionada:').first()).toBeVisible()
  })

  // ── template/documentation → content card ──────────────────

  test('template/documentation block renders as a collapsible card', async ({ page }) => {
    const s = suffix()
    const templatePage = `template/doc-${s}`
    const dataPage = `cards-doc-${s}`

    await createPage(page, templatePage)
    await createBlock(page, templatePage, '', {
      'card-shape': 'content',
      'icon': '📄',
    })

    await createBlock(page, dataPage, 'Pipelines docs', {
      'template': `doc-${s}`,
    })

    await goToPage(page, dataPage)
    const card = page.locator('[data-testid="card-renderer"][data-shape="content"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
    await expect(card).toHaveAttribute('data-template', `doc-${s}`)

    // Content cards have a collapse toggle
    const collapseBtn = card.getByRole('button', { name: /collapse section/i })
    await expect(collapseBtn).toBeVisible()
  })

  // ── template:: without a matching template page ────────────

  test('unknown template name falls back to normal block (no crash)', async ({ page }) => {
    const s = suffix()
    const dataPage = `cards-orphan-${s}`

    await createBlock(page, dataPage, 'Orphan block', {
      'template': `no-such-template-${s}`,
    })

    await goToPage(page, dataPage)
    await expect(page.getByText('Orphan block').first()).toBeVisible({ timeout: 10000 })

    // No CardRenderer wrapper — the template page is missing
    await expect(page.locator('[data-testid="card-renderer"]')).toHaveCount(0, { timeout: 5000 })
  })

  // ── Legacy fallback: type:: reference ──────────────────────

  test('type:: reference renders with console.warn fallback', async ({ page }) => {
    const s = suffix()
    const dataPage = `cards-legacy-ref-${s}`

    const warns: string[] = []
    page.on('console', msg => {
      if (msg.type() === 'warning' && msg.text().includes('legacy "type:: reference"')) {
        warns.push(msg.text())
      }
    })

    await createBlock(page, dataPage, 'Legacy ref', {
      'type': 'reference',
      'dda-relacionada': 'from legacy',
    })

    await goToPage(page, dataPage)
    const card = page.locator('[data-testid="card-renderer"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
    await expect(page.getByText('Legacy ref').first()).toBeVisible()
    expect(warns.length).toBeGreaterThanOrEqual(1)
  })

  // ── Legacy fallback: type:: documentacion ──────────────────

  test('type:: documentacion renders with console.warn fallback', async ({ page }) => {
    const s = suffix()
    const dataPage = `cards-legacy-doc-${s}`

    await createBlock(page, dataPage, 'Legacy doc', { 'type': 'documentacion' })

    await goToPage(page, dataPage)
    const card = page.locator('[data-testid="card-renderer"][data-shape="content"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
    await expect(page.getByText('Legacy doc').first()).toBeVisible()
  })

  // ── EmptyState shows TemplatePicker ────────────────────────

  test('EmptyState shows TemplatePicker with templates and creates a block', async ({ page }) => {
    const s = suffix()
    const templatePage = `template/tp-ref-${s}`
    const dataPage = `cards-tp-${s}`

    await createPage(page, templatePage)
    await createBlock(page, templatePage, '', {
      'card-shape': 'reference',
      'icon': '🔗',
    })

    await goToPage(page, dataPage)
    // Picker should be visible when there are templates
    const picker = page.locator('[data-testid="template-picker"]')
    await expect(picker).toBeVisible({ timeout: 15000 })

    // The template card is rendered in the picker
    await expect(page.locator(`[data-testid="template-card-tp-ref-${s}"]`)).toBeVisible({ timeout: 5000 })

    // Select and confirm
    await page.locator(`[data-testid="template-card-tp-ref-${s}"]`).click()
    const confirm = page.locator('[data-testid="template-picker-confirm"]')
    await expect(confirm).toBeEnabled()
    await confirm.click()

    await page.waitForTimeout(2000)

    // A card should appear with the template shape
    const card = page.locator('[data-testid="card-renderer"][data-shape="reference"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
  })

  // ── EmptyState fallback when no templates exist ─────────────

  test('EmptyState shows fallback button when no template pages', async ({ page }) => {
    const s = suffix()
    const dataPage = `cards-empty-${s}`

    await goToPage(page, dataPage)
    const empty = page.locator('[data-testid="template-picker-empty"]')
    await expect(empty).toBeVisible({ timeout: 15000 })

    const addBtn = empty.getByRole('button', { name: 'Add first block' })
    await expect(addBtn).toBeVisible()
    await addBtn.click()

    await page.waitForTimeout(1000)
    await expect(page.locator('main')).toBeVisible()
  })

  // ── CSS hooks for user-defined styling ─────────────────────

  test('card renders with cssclass and data-template attrs', async ({ page }) => {
    const s = suffix()
    const templatePage = `template/styled-${s}`
    const dataPage = `cards-styled-${s}`

    await createPage(page, templatePage)
    await createBlock(page, templatePage, '', {
      'card-shape': 'reference',
      'icon': '📋',
      'cssclass': 'card-meeting',
    })

    await createBlock(page, dataPage, 'Styled block', {
      'template': `styled-${s}`,
    })

    await goToPage(page, dataPage)
    const card = page.locator('[data-testid="card-renderer"][data-shape="reference"]').first()
    await expect(card).toBeVisible({ timeout: 10000 })
    await expect(card.getAttribute('class')).resolves.toContain('card-meeting')
    await expect(card).toHaveAttribute('data-template', `styled-${s}`)
  })
})
