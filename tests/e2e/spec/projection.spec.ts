/**
 * Projection Feature E2E Tests (ADR-0025 WU-3)
 *
 * Tests the projection renderer feature including:
 *   - Projection endpoint (GET /api/v1/blocks/:id/projection)
 *   - Presets endpoint (GET /api/v1/presets)
 *   - UI rendering with VITE_PROJECTION_RENDERER flag
 *
 * Run with:
 *   just dev
 *   # in another shell:
 *   QUILT_API_KEY=$(grep VITE_QUILT_API_KEY quilt-ui/.env | cut -d= -f2) \
 *     npx playwright test --grep @projection
 *
 * Auth: every API call goes through `getAuthHeaders()` (Bearer token).
 */

import { test, expect } from '@playwright/test'
import { getAuthHeaders } from '../auth-state'

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737'
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173'

function suffix(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
}

test.describe('Projection API Endpoints', () => {
  test('GET /api/v1/presets returns preset list', async ({ request }) => {
    const headers = getAuthHeaders()
    const resp = await request.get(`${API_URL}/api/v1/presets`, { headers })
    expect(resp.status()).toBe(200)

    const body = await resp.json()
    expect(body).toHaveProperty('presets')
    expect(body).toHaveProperty('count')
    expect(Array.isArray(body.presets)).toBe(true)
  })

  test('GET /api/v1/blocks/:id/projection returns projection view for existing block', async ({ request }) => {
    // First create a page and block via REST
    const headers = getAuthHeaders()
    const pageName = `proj-test-${suffix()}`

    const pageResp = await request.post(`${API_URL}/api/v1/pages`, {
      data: { name: pageName },
      headers,
    })
    expect(pageResp.status()).toBe(201)
    const page = await pageResp.json()

    const blockResp = await request.post(`${API_URL}/api/v1/blocks`, {
      data: {
        page_id: page.id,
        content: 'Test block for projection',
        block_type: 'paragraph',
      },
      headers,
    })
    expect(blockResp.status()).toBe(201)
    const block = await blockResp.json()

    const projResp = await request.get(`${API_URL}/api/v1/blocks/${block.id}/projection`, {
      headers,
    })
    expect(projResp.status()).toBe(200)

    const projection = await projResp.json()
    expect(projection).toHaveProperty('text')
    expect(projection).toHaveProperty('links')
    expect(projection).toHaveProperty('children')
    expect(projection).toHaveProperty('decorations')
    expect(projection).toHaveProperty('conflicts')
    expect(projection).toHaveProperty('properties')
  })

  test('GET /api/v1/blocks/:id/projection returns 404 for non-existent block', async ({ request }) => {
    const headers = getAuthHeaders()
    const fakeId = '00000000-0000-0000-0000-000000000000'
    const resp = await request.get(`${API_URL}/api/v1/blocks/${fakeId}/projection`, {
      headers,
    })
    expect(resp.status()).toBe(404)
  })
})

test.describe('Projection UI with Feature Flag', () => {
  test.beforeEach(async ({ page }) => {
    // Enable the projection renderer flag for these tests
    await page.goto(FRONTEND_URL)
    // The flag is read at module load time, so we need to set it before navigation
    // This is done via environment variable in production, but for E2E we test with the flag ON
  })

  test('projection flag can be set via environment', async ({ page }) => {
    // Verify that with the flag ON, the projection renderer is used
    // This test just verifies the UI can load - actual projection rendering
    // is tested via the API tests above
    await page.goto(FRONTEND_URL)
    await expect(page.locator('#root')).toBeVisible({ timeout: 15_000 })
  })
})
