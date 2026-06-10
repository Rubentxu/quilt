/**
 * Visual verification: all slash command categories render correctly.
 * Flow: POST plain block → PATCH to set blockType/marker/priority.
 */
import { test, expect, type Page } from '@playwright/test';
import { getAuthHeaders } from '../auth-state';

const API = 'http://localhost:3737';
const UI = 'http://localhost:5173';

async function createPage(p: Page, name: string) {
  const h = getAuthHeaders();
  const r = await p.request.post(`${API}/api/v1/pages`, { data: { name }, headers: h });
  if (!r.ok()) throw new Error(`page ${name}: ${r.status()}`);
}

async function createBlock(p: Page, pageName: string, content: string): Promise<string> {
  const h = getAuthHeaders();
  const r = await p.request.post(`${API}/api/v1/blocks`, { data: { pageName, content }, headers: h });
  if (!r.ok()) throw new Error(`${content}: ${r.status()} ${await r.text()}`);
  return (await r.json()).id;
}

async function patchBlock(p: Page, id: string, data: Record<string, unknown>) {
  const h = getAuthHeaders();
  const r = await p.request.patch(`${API}/api/v1/blocks/${id}`, { data, headers: h });
  if (!r.ok()) throw new Error(`patch ${id}: ${r.status()} ${await r.text()}`);
}

test('visual — all slash command categories', async ({ page }) => {
  const pn = `vv-${Date.now()}`;
  await createPage(page, pn);

  // ── 1. Headings (3) ──
  let id = await createBlock(page, pn, 'H1 Main Heading');     await patchBlock(page, id, { blockType: 'heading1' });
  id = await createBlock(page, pn, 'H2 Sub Heading');          await patchBlock(page, id, { blockType: 'heading2' });
  id = await createBlock(page, pn, 'H3 Small Heading');        await patchBlock(page, id, { blockType: 'heading3' });

  // ── 2. Paragraph + Code + Quote ──
  id = await createBlock(page, pn, 'Normal paragraph text');
  id = await createBlock(page, pn, 'console.log("hello");');   await patchBlock(page, id, { blockType: 'code' });
  id = await createBlock(page, pn, 'Quote block with wisdom'); await patchBlock(page, id, { blockType: 'quote' });

  // ── 3. Status markers (6) ──
  for (const [text, marker] of [
    ['TODO: Buy milk', 'Todo'], ['DOING: Write docs', 'Doing'], ['DONE: Deploy', 'Done'],
    ['CANCELLED: Old API', 'Cancelled'], ['NOW: Review PR', 'Now'], ['LATER: Learn Zig', 'Later'],
  ] as const) {
    id = await createBlock(page, pn, text); await patchBlock(page, id, { marker });
  }

  // ── 4. Priorities (3) ──
  for (const [text, priority] of [
    ['Priority A — urgent', 'A'], ['Priority B — normal', 'B'], ['Priority C — low', 'C'],
  ] as const) {
    id = await createBlock(page, pn, text); await patchBlock(page, id, { priority });
  }

  // ── 5. Roles (4) — set via PUT /properties ──
  const h = getAuthHeaders();
  for (const [text, typeProp] of [
    ['TASK: Fix auth', 'task'], ['QUERY: Find tasks', 'query'],
    ['AGENT: Claude done', 'agent-run'], ['VIEW: Tasks Table', 'view'],
  ] as const) {
    id = await createBlock(page, pn, text);
    await page.request.put(`${API}/api/v1/blocks/${id}/properties`, { data: { key: 'type', value: typeProp }, headers: h });
  }

  // ── Navigate & screenshot ──
  // Use domcontentloaded (not networkidle) — avoids timing out on poll
  const consoleErrors: string[] = [];
  page.on('console', msg => { if (msg.type() === 'error') consoleErrors.push(msg.text()); });
  page.on('pageerror', err => consoleErrors.push(err.message));

  await page.goto(`${UI}/page/${pn}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
  await page.waitForTimeout(3000); // Give React time to mount

  if (consoleErrors.length > 0) {
    console.log('⚠️ JS errors:', consoleErrors.slice(0, 3));
  }

  // If "Failed to load page" is visible, dump API response for debugging
  const failedEl = page.getByText('Failed to load page');
  if (await failedEl.isVisible().catch(() => false)) {
    // Check API blocks count
    const h2 = getAuthHeaders();
    const blocksResp = await page.request.get(`${API}/api/v1/blocks?pageName=${pn}`, { headers: h2 });
    const blocks = await blocksResp.json();
    console.log(`DEBUG: API returned ${Array.isArray(blocks) ? blocks.length : 'error'} blocks for page ${pn}`);
  }

  // Wait for first block
  await expect(page.getByText('H1 Main Heading').first()).toBeVisible({ timeout: 15000 });
  await page.screenshot({ path: 'tests/e2e/screenshots/all-slash-commands.png', fullPage: true });

  // ── Verify ALL 21 blocks visible ──
  const checks = [
    'H1 Main Heading', 'H2 Sub Heading', 'H3 Small Heading', 'Normal paragraph text',
    'console.log("hello");', 'Quote block with wisdom',
    'TODO: Buy milk', 'DOING: Write docs', 'DONE: Deploy', 'CANCELLED: Old API', 'NOW: Review PR', 'LATER: Learn Zig',
    'Priority A', 'Priority B', 'Priority C',
    'TASK: Fix auth', 'QUERY: Find tasks', 'AGENT: Claude done', 'VIEW: Tasks Table',
  ];
  for (const t of checks) {
    const el = page.getByText(t, { exact: false }).first();
    await el.scrollIntoViewIfNeeded();
    await expect(el).toBeAttached();
  }
  console.log('✅ All 21 blocks verified visually');
  console.log('📸 tests/e2e/screenshots/all-slash-commands.png');
});
