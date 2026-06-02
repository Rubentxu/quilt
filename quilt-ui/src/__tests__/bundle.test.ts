// Bundle size guardrails.
//
// These tests enforce the performance budget documented in
// docs/PERFORMANCE.md. They run against the production build
// produced by `npm run build` — make sure to build before running
// `npm test`, or the dist/ directory won't exist.
//
// If a change pushes the bundle over budget, the test fails with a
// clear message pointing to the offending chunk. The intent is to
// catch regressions before they land, not to police every kilobyte.

import { describe, it, expect, beforeAll } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import zlib from 'node:zlib'

// ─── Helpers ─────────────────────────────────────────────────────

const DIST = path.join(__dirname, '..', '..', 'dist', 'assets')

/** List every file in the dist/assets directory. */
function listAssets(): string[] {
  if (!fs.existsSync(DIST)) return []
  return fs.readdirSync(DIST)
}

/** Read raw bytes and return both raw and gzipped sizes. */
function sizes(file: string): { raw: number; gz: number } {
  const buf = fs.readFileSync(path.join(DIST, file))
  const gz = zlib.gzipSync(buf, { level: 9 })
  return { raw: buf.length, gz: gz.length }
}

/** Pick the first file matching a regex prefix. */
function findAsset(prefix: RegExp): string | undefined {
  return listAssets().find(f => prefix.test(f))
}

// ─── Tests ───────────────────────────────────────────────────────

describe('Bundle budget', () => {
  // Skip the entire suite gracefully if there's no dist/. The tests
  // are only meaningful against a production build, not in isolation.
  beforeAll(() => {
    if (!fs.existsSync(DIST)) {
      throw new Error(
        `dist/assets not found at ${DIST}. Run \`npm run build\` first.`,
      )
    }
  })

  it('emits a reasonable number of chunks (code splitting is active)', () => {
    const files = listAssets()
    // 1 css + 1 wasm + several JS chunks. Fewer than 8 means we
    // probably regressed to a single bundle.
    expect(files.length).toBeGreaterThanOrEqual(8)
  })

  it('main entry bundle (index-*.js) is under 50 kB gzipped', () => {
    const main = findAsset(/^index-.*\.js$/)
    expect(main, 'main entry chunk not found').toBeDefined()
    const { gz } = sizes(main!)
    // 50 kB gzipped is the post-optimization target. The original
    // single bundle was ~133 kB gzipped.
    expect(
      gz,
      `main entry is ${gz} bytes gzipped (budget: 50 kB)`,
    ).toBeLessThan(50 * 1024)
  })

  it('React vendor chunk exists and is under 80 kB gzipped', () => {
    const react = findAsset(/^react-vendor-.*\.js$/)
    expect(react, 'react-vendor chunk missing — vite manualChunks may have regressed').toBeDefined()
    const { gz } = sizes(react!)
    expect(gz, `react-vendor is ${gz} bytes`).toBeLessThan(80 * 1024)
  })

  it('Router vendor chunk exists and is under 40 kB gzipped', () => {
    const router = findAsset(/^router-vendor-.*\.js$/)
    expect(router, 'router-vendor chunk missing').toBeDefined()
    const { gz } = sizes(router!)
    expect(gz, `router-vendor is ${gz} bytes`).toBeLessThan(40 * 1024)
  })

  it('WASM blob is under 600 kB gzipped', () => {
    const wasm = findAsset(/^quilt_core_bg-.*\.wasm$/)
    expect(wasm, 'WASM blob missing').toBeDefined()
    const { gz } = sizes(wasm!)
    // Current build: ~462 kB. Allow some headroom for the engine
    // to grow, but anything over 600 kB warrants a look.
    expect(gz, `WASM is ${gz} bytes`).toBeLessThan(600 * 1024)
  })

  it('initial-load gzipped JS is under 150 kB', () => {
    // The critical-path chunks are: index-*.js, react-vendor-*.js,
    // router-vendor-*.js, icons-vendor-*.js, toast-vendor-*.js,
    // vendor-misc-*.js, plus the active route chunk (HomePage).
    const initialPrefixes = [
      /^index-.*\.js$/,
      /^react-vendor-.*\.js$/,
      /^router-vendor-.*\.js$/,
      /^icons-vendor-.*\.js$/,
      /^toast-vendor-.*\.js$/,
      /^vendor-misc-.*\.js$/,
      /^HomePage-.*\.js$/,
    ]
    let total = 0
    for (const re of initialPrefixes) {
      const file = findAsset(re)
      if (file) total += sizes(file).gz
    }
    // 150 kB is the pre-optimization baseline minus ~10%. The point
    // is to ensure no single chunk or new dependency pushes us back
    // above the legacy single-bundle number.
    expect(
      total,
      `initial JS totals ${total} bytes gzipped (budget: 150 kB)`,
    ).toBeLessThan(150 * 1024)
  })

  it('lazy routes are in their own chunks (≤ 10 kB gzipped each)', () => {
    const lazyRoutePrefixes = [
      /^PageViewPage-.*\.js$/,
      /^JournalPage-.*\.js$/,
      /^SettingsPage-.*\.js$/,
      /^AllPagesPage-.*\.js$/,
      /^GraphViewPage-.*\.js$/,
    ]
    for (const re of lazyRoutePrefixes) {
      const file = findAsset(re)
      expect(file, `expected lazy route chunk matching ${re}`).toBeDefined()
      if (file) {
        const { gz } = sizes(file)
        // Route shell files should be tiny — they only contain the
        // loader + props unwrap. The real work lives in nested chunks
        // (PageView, SlashCommandMenu, etc.).
        expect(
          gz,
          `${file} is ${gz} bytes gzipped — should be < 10 kB`,
        ).toBeLessThan(10 * 1024)
      }
    }
  })
})
