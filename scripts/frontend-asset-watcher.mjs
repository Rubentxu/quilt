#!/usr/bin/env node
// frontend-asset-watcher.mjs — Watches quilt-ui/src/ for changes and
// triggers a fresh Vite production build, syncing the output into
// crates/quilt-server/wasm_assets/.
//
// This keeps the server (port 3737) serving the latest frontend, so
// users who hit 3737 directly (e.g. inside a container) get fresh
// code on every reload without manual `vite build` invocations.

import { watch } from 'node:fs'
import { spawn } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(__dirname, '..')
const WATCH_DIR = resolve(ROOT, 'quilt-ui', 'src')

let rebuildTimer = null
let isBuilding = false
let pendingRebuild = false

function log(msg) {
  const time = new Date().toLocaleTimeString('es-ES', { hour12: false })
  process.stdout.write(`[frontend-watcher ${time}] ${msg}\n`)
}

function triggerRebuild() {
  if (rebuildTimer) clearTimeout(rebuildTimer)
  // Debounce 500ms — wait until the user pauses typing
  rebuildTimer = setTimeout(runRebuild, 500)
}

function runRebuild() {
  if (isBuilding) {
    pendingRebuild = true
    return
  }
  isBuilding = true
  log('rebuilding…')
  const start = Date.now()
  const build = spawn('npx', ['vite', 'build'], {
    cwd: resolve(ROOT, 'quilt-ui'),
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let stderr = ''
  build.stderr.on('data', (d) => { stderr += d.toString() })
  build.on('close', (code) => {
    if (code !== 0) {
      log(`✗ build failed (exit ${code})\n${stderr.split('\n').slice(-5).join('\n')}`)
    } else {
      const sync = spawn('bash', [resolve(ROOT, 'scripts', 'sync-frontend-assets.sh')], {
        stdio: 'inherit',
      })
      sync.on('close', () => {
        const ms = Date.now() - start
        log(`✓ wasm_assets/ updated (${ms}ms)`)
      })
    }
    isBuilding = false
    if (pendingRebuild) {
      pendingRebuild = false
      runRebuild()
    }
  })
}

// ── Recursive fs.watch with debounce ──────────────────────────────
function watchRecursive(dir) {
  try {
    const watcher = watch(dir, { recursive: true }, (event, filename) => {
      if (!filename) return
      // Only watch source files; ignore generated / node_modules
      if (filename.includes('node_modules')) return
      if (filename.includes('dist')) return
      if (filename.includes('.git')) return
      if (filename.endsWith('.swp') || filename.endsWith('~')) return
      log(`change: ${filename}`)
      triggerRebuild()
    })
    watcher.on('error', (err) => {
      log(`watch error: ${err.message}`)
    })
    return watcher
  } catch (err) {
    log(`could not watch ${dir}: ${err.message}`)
    return null
  }
}

log(`watching ${WATCH_DIR}`)
const w = watchRecursive(WATCH_DIR)
if (!w) process.exit(1)

// Initial build to make sure wasm_assets/ has something
triggerRebuild()

process.on('SIGINT', () => process.exit(0))
process.on('SIGTERM', () => process.exit(0))
