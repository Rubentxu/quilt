/**
 * getBlockCard — ADR-0007
 *
 * Resolves a block to its visual card definition. Replaces the
 * hardcoded `getBlockType()` switch on `type::` with a data-driven
 * lookup against the `template::` property.
 *
 * Algorithm:
 *   1. If block has `template:: <name>`, look up that template page
 *      and return its `card-shape::` plus `icon::` and `cssclass::`.
 *   2. FALLBACK (V1 only): if block has legacy `type:: reference` or
 *      `type:: documentacion`, return the equivalent card-shape and
 *      warn. The user should migrate to `template::` over time.
 *   3. Otherwise return null (block renders as a normal outliner row).
 *
 * The `allTemplates` map is built by PageView from all `template/*`
 * pages, keyed by template name. The map is passed in so this helper
 * stays pure and testable.
 */

import type { Block, BlockProperty, Page } from '@shared/types/api'
import type { BlockCard, CardShape } from './CardRenderer'

// ── Property extraction helpers ───────────────────────────────────

/** Read a string property by key. Returns undefined if missing/not a string. */
function getStringProperty(
  properties: BlockProperty[] | undefined,
  key: string,
): string | undefined {
  const prop = properties?.find(p => p.key === key)
  if (!prop) return undefined
  const value = prop.value
  return typeof value === 'string' ? value : undefined
}

/**
 * Extract the visual metadata for a template page.
 *
 * A template page is a regular page that:
 *   - has a `template/` prefix in its name
 *   - declares a `card-shape::` property (optional — defaults to 'inline')
 *   - may declare `icon::` and `cssclass::` for decoration
 *
 * Returns null if the page is not a template page.
 */
export function getTemplateCardFromPage(
  page: Page,
  properties: BlockProperty[] | undefined,
): BlockCard | null {
  // Only pages with the template/ prefix are templates
  if (!page.name.startsWith('template/')) return null

  const shapeRaw = getStringProperty(properties, 'card-shape')
  const shape: CardShape =
    shapeRaw === 'reference' || shapeRaw === 'content' || shapeRaw === 'inline'
      ? shapeRaw
      : 'inline'

  return {
    shape,
    icon: getStringProperty(properties, 'icon'),
    cssclass: getStringProperty(properties, 'cssclass'),
    // The display name is the page's short name (without the `template/` prefix)
    templateName: page.name.replace(/^template\//, '') || page.name,
  }
}

// ── Main resolver ──────────────────────────────────────────────────

/**
 * Resolve a block to its card definition.
 *
 * @param block       The block being rendered
 * @param allTemplates Map of template name (without `template/` prefix) →
 *                    `BlockCard`. Built by the caller from the
 *                    `api.listPages()` results filtered to `template/*`.
 * @returns The card to render, or null for a normal outliner block.
 */
export function getBlockCard(
  block: Block,
  allTemplates: Map<string, BlockCard>,
): BlockCard | null {
  const props = block.properties as BlockProperty[] | undefined

  // 1. Primary path: `template::` property
  const templateName = getStringProperty(props, 'template')
  if (templateName) {
    const card = allTemplates.get(templateName)
    if (card) return card
    // Block has `template:: <name>` but the template page is missing.
    // Warn but don't break — the block still renders as a normal row.
    // eslint-disable-next-line no-console
    console.warn(
      `[getBlockCard] Block ${block.id} references unknown template "${templateName}". Falling back to inline.`,
    )
    return null
  }

  // 2. Legacy fallback: `type:: reference` / `type:: documentacion`
  // Kept in V1 to avoid breaking existing data. A `console.warn` reminds
  // the developer/migrator to update the block to the new format.
  const legacyType = getStringProperty(props, 'type')
  if (legacyType === 'reference') {
    // eslint-disable-next-line no-console
    console.warn(
      `[getBlockCard] Block ${block.id} uses legacy "type:: reference". Migrate to "template:: <name>" with "card-shape:: reference".`,
    )
    return {
      shape: 'reference',
      templateName: 'reference',
    }
  }
  if (legacyType === 'documentacion' || legacyType === 'documentation') {
    // The codebase had two spellings: `documentacion` (Spanish, no accent,
    // original) and `documentation` (English, used in some tests). Both
    // are accepted here for backward compatibility.
    // eslint-disable-next-line no-console
    console.warn(
      `[getBlockCard] Block ${block.id} uses legacy "type:: ${legacyType}". Migrate to "template:: <name>" with "card-shape:: content".`,
    )
    return {
      shape: 'content',
      templateName: 'documentation',
    }
  }

  // 3. No template activation — normal outliner block
  return null
}

/**
 * Build the `allTemplates` map by fetching all `template/*` pages and
 * reading their card metadata. Called once per page render.
 *
 * NOTE: this is a pure helper — the caller is responsible for fetching
 * the pages from `api.listPages()`. We keep it pure so it can be tested
 * with fixture data.
 */
export function buildTemplateIndex(
  pages: Page[],
  propertiesByPageId: Map<string, BlockProperty[]>,
): Map<string, BlockCard> {
  const index = new Map<string, BlockCard>()
  for (const page of pages) {
    if (!page.name.startsWith('template/')) continue
    const props = propertiesByPageId.get(page.id)
    const card = getTemplateCardFromPage(page, props)
    if (!card) continue
    const shortName = page.name.replace(/^template\//, '')
    index.set(shortName, card)
  }
  return index
}

// ── Block metas helper ─────────────────────────────────────────────

/**
 * Returns all block properties EXCEPT the ones used for card activation
 * (`template`, `type`, `card-shape`, `icon`, `cssclass`), formatted as
 * metas for display inside the card.
 *
 * Replaces the original `getBlockMetas()` which only excluded `type`.
 */
export function getCardMetas(block: Block): { key: string; value: string }[] {
  const RESERVED_KEYS = new Set([
    'template',
    'type',
    'card-shape',
    'icon',
    'cssclass',
    'collapsed', // block-level collapse is rendered by the outliner, not the card
  ])
  return (block.properties ?? [])
    .filter((p): p is BlockProperty & { value: string } =>
      !RESERVED_KEYS.has(p.key) && typeof p.value === 'string',
    )
    .map(p => ({ key: p.key, value: p.value }))
}
