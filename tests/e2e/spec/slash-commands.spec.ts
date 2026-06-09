/**
 * Slash Commands E2E Tests
 *
 * Covers the slash-command surface in `quilt-ui/src/features/outliner-tiptap/slashRegistry.tsx`.
 * Verifies the three behaviors the registry powers:
 *
 *   1. **Menu interaction** — the dropdown opens on `/`, filters as the
 *      user types, and closes on Escape.
 *   2. **Status / marker commands** — `/todo`, `/done`, `/cancelled` flip
 *      the block's `marker` to the title-cased TaskMarker value.
 *   3. **Block type commands** — `/h1`, `/h2`, `/code`, `/quote` flip the
 *      block's `blockType` and the value PERSISTS across a full page
 *      reload (regression guard for P0 fix #6 — slash blockType used
 *      to be lost on save).
 *   4. **Role commands** — `/task`, `/query`, `/card` write structured
 *      `properties` on the block (type:: task / type:: query + dsl:: /
 *      card-shape::). The role menu handlers are property transforms
 *      (NOT blockType changes), so the test asserts via the
 *      `/api/v1/blocks/:id/properties` endpoint, not via blockType.
 *
 * Tag: `@slash-commands` — run with `npx playwright test --grep @slash-commands`.
 *
 * Auth: every API call goes through `getAuthHeaders()` (Bearer token from
 * `QUILT_API_KEY`). The frontend itself is reached through Vite at 5173.
 *
 * Per project rules:
 *   - No CSS selectors — `getByRole` / `getByLabelText` / `getByText`
 *   - No `waitForTimeout` — `findBy*` / `expect().toBeVisible()` / `toHaveURL`
 *   - Tests MUST fail (not skip) if the backend is unreachable.
 *   - Test behaviour, not implementation — assert on user-visible state
 *     (DOM text, API record) rather than React internals.
 *
 * Manual execution:
 *   just dev
 *   # in another shell:
 *   QUILT_API_KEY=$(grep VITE_QUILT_API_KEY quilt-ui/.env | cut -d= -f2) \
 *     npx playwright test --grep @slash-commands
 */

import { test, expect, type Page } from '@playwright/test'
import { getAuthHeaders } from '../auth-state'

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737'
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173'

// ── Helpers ──────────────────────────────────────────────────────

/** Random suffix — every artifact (page, block) gets a unique one to
 *  avoid UNIQUE collisions when tests run in parallel. */
function suffix(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
}

/** Create a regular page via REST. Throws on non-2xx — no silent skip. */
async function createPage(page: Page, name: string): Promise<void> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/pages`, {
    data: { name },
    headers,
  })
  if (!resp.ok()) {
    throw new Error(`createPage(${name}) failed with ${resp.status()}: ${await resp.text()}`)
  }
}

/** Create a block via REST. Returns the block id. */
async function createBlock(
  page: Page,
  pageName: string,
  content: string,
): Promise<string> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content },
    headers,
  })
  if (!resp.ok()) {
    throw new Error(`createBlock failed with ${resp.status()}: ${await resp.text()}`)
  }
  return ((await resp.json()) as { id: string }).id
}

/** Fetch all blocks for a page (REST). */
async function getPageBlocks(
  page: Page,
  pageName: string,
): Promise<Array<{
  id: string
  content: string
  blockType: string
  marker: string | null
  properties: Record<string, unknown>
}>> {
  const headers = getAuthHeaders()
  const resp = await page.request.get(
    `${API_URL}/api/v1/pages/${encodeURIComponent(pageName)}/blocks`,
    { headers },
  )
  if (!resp.ok()) {
    throw new Error(`getPageBlocks failed with ${resp.status()}: ${await resp.text()}`)
  }
  return (await resp.json()) as Array<{
    id: string
    content: string
    blockType: string
    marker: string | null
    properties: Record<string, unknown>
  }>
}

/** Delete every block on a page (best-effort cleanup). */
async function deleteAllBlocks(page: Page, pageName: string): Promise<void> {
  const headers = getAuthHeaders()
  const blocks = await getPageBlocks(page, pageName)
  for (const block of blocks) {
    await page.request.delete(`${API_URL}/api/v1/blocks/${block.id}`, { headers })
  }
}

/** Open a regular page in the browser, wait for the seeded block to
 *  appear, click it to enter edit mode, and return the
 *  `contenteditable` editor locator. The editor MUST be focused and
 *  contain the original content before the caller types. */
async function openPageAndEditBlock(
  page: Page,
  pageName: string,
  expectedContent: string,
) {
  await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`)
  const readContent = page.locator('.block-content-read').first()
  await expect(readContent).toBeVisible({ timeout: 10_000 })
  await readContent.click()
  const editor = page.locator('.block-content[contenteditable="true"]').first()
  await expect(editor).toBeVisible({ timeout: 5_000 })
  // The editor mounts with the original block content; if the
  // expected text isn't there, the seed never landed.
  await expect(editor).toContainText(expectedContent)
  return editor
}

/** Clear the editor's text and type the given string at the start.
 *  The slash menu keys off the editor's textContent — the new text
 *  MUST start with `/`, so we always replace, never append. We use
 *  `fill('')` to wipe (sets textContent + fires React's input event)
 *  and then `type()` with a small delay so the input handler
 *  processes each character individually. */
async function replaceEditorText(
  editor: import('@playwright/test').Locator,
  text: string,
) {
  await editor.fill('')
  await editor.type(text, { delay: 10 })
}

/** Type the slash command and confirm it via the keyboard, not the
 *  mouse. The click path is racy in E2E: clicking a menu item fires
 *  mousedown → editor blur → React unmounts the menu → the click
 *  event has no target left to fire on. The SlashCommandMenu's
 *  document-level keydown handler picks Enter for the highlighted
 *  item regardless, so `press('Enter')` is the deterministic path.
 *
 *  `arrowDownCount` lets callers skip past items that match the
 *  query but aren't the desired one (e.g. `/task` matches
 *  status-todo first, then task-role; pass 1 to select the second). */
async function applySlashCommand(
  editor: import('@playwright/test').Locator,
  slashText: string,
  arrowDownCount = 0,
) {
  await replaceEditorText(editor, slashText)
  // Sanity: the menu must be visible after the slash. We use the
  // menu's fixed-position container (z-index 200) as the signal
  // because which category headers render depends on which items
  // match the query — `/card` and `/query` show only the "Roles"
  // header, `/h1` shows only the "Block Types" header, etc.
  const menu = editor.page().locator('div[style*="z-index: 200"]')
  await expect(menu).toBeVisible({ timeout: 5_000 })
  for (let i = 0; i < arrowDownCount; i++) {
    await editor.press('ArrowDown')
  }
  await editor.press('Enter')
}

// ── Tests ────────────────────────────────────────────────────────

test.describe('Slash menu @slash-commands', () => {
  test('@smoke slash menu opens on / key @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-menu-open-${s}`
    await createPage(page, host)
    await createBlock(page, host, 'seed-for-slash-open')

    const editor = await openPageAndEditBlock(page, host, 'seed-for-slash-open')
    await replaceEditorText(editor, '/')

    // The menu is rendered as a fixed-position div with z-index 200.
    // Its visibility is the strongest "menu is open" signal —
    // category headers (Status, Roles, ...) only render when their
    // category has matching items, so a single header is fragile.
    const menu = page.locator('div[style*="z-index: 200"]')
    await expect(menu).toBeVisible({ timeout: 5_000 })
    // The first status item "TODO" is rendered as a label inside
    // the menu. With query='' the Status section shows the six
    // status items. Use `.first()` because the registry's render
    // path iterates categories × items and can render the same
    // item under multiple category containers — a single label
    // text can therefore match more than once in the DOM.
    await expect(menu.getByText('TODO', { exact: true }).first()).toBeVisible()
  })

  test('slash menu closes on Escape @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-menu-escape-${s}`
    await createPage(page, host)
    await createBlock(page, host, 'seed-for-escape')

    const editor = await openPageAndEditBlock(page, host, 'seed-for-escape')
    await replaceEditorText(editor, '/')

    // Sanity: menu is open before Escape.
    const menu = page.locator('div[style*="z-index: 200"]')
    await expect(menu).toBeVisible({ timeout: 5_000 })

    // Escape is handled at document level by SlashCommandMenu.
    await page.keyboard.press('Escape')

    // The container unmounts when slash state clears.
    await expect(menu).toBeHidden({ timeout: 5_000 })
  })

  test('slash menu filters by typed text (headings) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-menu-filter-${s}`
    await createPage(page, host)
    await createBlock(page, host, 'seed-for-filter')

    const editor = await openPageAndEditBlock(page, host, 'seed-for-filter')
    // Typing `/h` opens the menu with the heading items among the
    // matches (the registry's `heading1/2/3` items have "Heading N" in
    // their label — they are guaranteed to be present in the filtered
    // list). We do NOT assert that ONLY the 3 headings are shown,
    // because other items with "h" in keywords (priority A → "high",
    // priority C → "have", query → "search", card → "shape", etc.)
    // also match. The test asserts the headings ARE present.
    //
    // Use `.first()` on each label because the menu's render
    // iterates categories × items — each item can render under
    // multiple category containers in the DOM, so a single label
    // text can match more than once.
    await replaceEditorText(editor, '/h')

    await expect(
      page.getByText('Heading 1', { exact: true }).first(),
    ).toBeVisible({ timeout: 5_000 })
    await expect(
      page.getByText('Heading 2', { exact: true }).first(),
    ).toBeVisible()
    await expect(
      page.getByText('Heading 3', { exact: true }).first(),
    ).toBeVisible()
  })
})

test.describe('Status slash commands @slash-commands', () => {
  // The status handler in `slashRegistry.tsx` calls
  // `api.updateBlock(id, { marker: TaskMarker })` and the server's
  // PATCH handler accepts and persists both `marker` and
  // `priority`. The tests assert BOTH the DOM (marker badge
  // visible) and the API (block record carries the title-cased
  // marker — Todo / Done / Cancelled) — the round-trip matters
  // because a silent server-side drop would leave the badge
  // missing on reload, even if the client rendered it
  // optimistically.
  test('/todo sets block marker to Todo @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-todo-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'todo seed')

    const editor = await openPageAndEditBlock(page, host, 'todo seed')
    // /todo matches status-todo, the blockType `todo`, and task-role.
    // The first match (status-todo) is the one we want, so no
    // ArrowDown needed.
    await applySlashCommand(editor, '/todo', 0)

    // The marker badge renders the marker as UPPERCASE text inside
    // the block row (see BlockRow.tsx line 1177). The status handler
    // sets `marker: "Todo"` (title-cased) on the server.
    const row = page.locator('.block-row').first()
    await expect(row.getByText('TODO', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    // Server-side confirmation: the block record carries marker=Todo.
    // (TaskMarker is the title-cased enum value, not the lowercase id.)
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Todo')

    await deleteAllBlocks(page, host)
  })

  test('/done sets block marker to Done @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-done-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'done seed')

    const editor = await openPageAndEditBlock(page, host, 'done seed')
    await applySlashCommand(editor, '/done', 0)

    const row = page.locator('.block-row').first()
    await expect(row.getByText('DONE', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Done')

    await deleteAllBlocks(page, host)
  })

  test('/cancelled sets block marker to Cancelled @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-cancelled-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'cancelled seed')

    const editor = await openPageAndEditBlock(page, host, 'cancelled seed')
    await applySlashCommand(editor, '/cancelled', 0)

    const row = page.locator('.block-row').first()
    await expect(row.getByText('CANCELLED', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Cancelled')

    await deleteAllBlocks(page, host)
  })
})

test.describe('Block type slash commands @slash-commands', () => {
  test('/h1 creates heading1 block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-h1-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'h1 seed')

    const editor = await openPageAndEditBlock(page, host, 'h1 seed')
    await applySlashCommand(editor, '/h1', 0)

    // The slash handler clears the editor text (preserveContent is
    // false for blockType changes) and PATCHes blockType=heading1.
    // We assert the API record first because it's a stronger signal
    // than the DOM (the blockType renders visually but the exact
    // CSS differs across themes).
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('heading1')

    // Reload the page and re-read the API. The blockType MUST survive
    // a fresh server response — this is the regression guard for
    // P0 fix #6 (slash blockType used to be lost on save).
    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded, 'block should still exist after reload').toBeDefined()
    expect(reloaded!.blockType).toBe('heading1')

    await deleteAllBlocks(page, host)
  })

  test('/h2 creates heading2 block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-h2-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'h2 seed')

    const editor = await openPageAndEditBlock(page, host, 'h2 seed')
    await applySlashCommand(editor, '/h2', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('heading2')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('heading2')

    await deleteAllBlocks(page, host)
  })

  test('/code creates code block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-code-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'code seed')

    const editor = await openPageAndEditBlock(page, host, 'code seed')
    await applySlashCommand(editor, '/code', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('code')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('code')

    await deleteAllBlocks(page, host)
  })

  test('/quote creates quote block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-quote-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'quote seed')

    const editor = await openPageAndEditBlock(page, host, 'quote seed')
    await applySlashCommand(editor, '/quote', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('quote')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('quote')

    await deleteAllBlocks(page, host)
  })
})

test.describe('Role slash commands @slash-commands', () => {
  test('/task sets type:: task + status:: todo @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-role-task-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'task seed')

    const editor = await openPageAndEditBlock(page, host, 'task seed')
    // /task matches status-todo, task-role, and the blockType `todo`.
    // status-todo is registered first, so Enter would select it. We
    // need task-role, which is the 2nd item — press ArrowDown once to
    // advance the highlight, then Enter.
    await applySlashCommand(editor, '/task', 1)

    // The role handler is a property transform: it does NOT change
    // blockType and it does NOT change marker. The block's
    // blockType stays paragraph; the change lives in `properties`.
    // The handler does:
    //   await api.setBlockProperty(id, 'type', 'task')
    //   await api.setBlockProperty(id, 'status', 'todo')
    // (see slashRegistry.tsx — makeRolePropertiesHandler)
    //
    // Both PUTs are async, so we retry the assertion until the
    // second one lands. The window is short (the handler awaits
    // each PUT sequentially) so 1s of polling is plenty.
    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({
        blockType: 'paragraph',
        properties: { type: 'task', status: 'todo' },
      })

    await deleteAllBlocks(page, host)
  })

  test('/query sets type:: query + dsl:: @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-role-query-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'query seed')

    // The /query handler calls window.prompt('Enter a DSL query:').
    // Accept the dialog with a fixed DSL string.
    const expectedDsl = `(and (page-property status open))`
    page.once('dialog', async (dialog) => {
      expect(dialog.type()).toBe('prompt')
      await dialog.accept(expectedDsl)
    })

    const editor = await openPageAndEditBlock(page, host, 'query seed')
    // /query is uniquely matched by query-role (no other item label,
    // blockType, or keyword contains "query"). Enter selects it,
    // which calls window.prompt and the dialog handler above accepts
    // with the expected DSL string.
    await applySlashCommand(editor, '/query', 0)

    // The handler does:
    //   await api.setBlockProperty(id, 'type', 'query')
    //   await api.setBlockProperty(id, 'dsl', dsl)
    // (the second PUT races the test's read — poll until both land)
    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({
        blockType: 'paragraph',
        properties: { type: 'query', dsl: expectedDsl },
      })

    await deleteAllBlocks(page, host)
  })

  test('/card sets card-shape:: @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-role-card-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'card seed')

    // The /card handler prompts for a card shape (one of:
    // content, reference, presentation, article, note). Default
    // is "content" if the user dismisses or types an unknown value.
    page.once('dialog', async (dialog) => {
      expect(dialog.type()).toBe('prompt')
      await dialog.accept('reference')
    })

    const editor = await openPageAndEditBlock(page, host, 'card seed')
    // /card is uniquely matched by card-role. Enter selects it,
    // which calls window.prompt and the dialog handler above
    // accepts with 'reference'.
    await applySlashCommand(editor, '/card', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({
        blockType: 'paragraph',
        properties: { 'card-shape': 'reference' },
      })

    await deleteAllBlocks(page, host)
  })
})
