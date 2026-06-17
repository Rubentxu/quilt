// ─── import/MigrationPanel — GS-9 manual resource ingestion ──────────────────
//
// RightSidebarSection panel for scanning, previewing, and ingesting/reindexing
// Markdown files within the active graph root directory.
//
// ## Flow
// 1. User clicks "Scan for files" — GET /api/v1/migration/candidates
// 2. Results displayed as a table showing each file's status (new/modified/skipped)
// 3. User clicks "Ingest" (new files) or "Reindex" (modified files)
// 4. Confirmation appears — explicit user action required (INV-3)
// 5. Operation runs — results displayed with per-file status

import { memo, useState, useCallback, useRef } from 'react'
import type { IngestionPlan, MigrationResponse, CandidateResult } from './types'
import { api } from '@core/api-client'

// ─── Constants ───────────────────────────────────────────────────────────────

const SECTION_ID = 'migration'
const DEFAULT_DEPTH = 8

// ─── Sub-components ──────────────────────────────────────────────────────────

/** Status badge with colour coding. */
function StatusBadge({ status }: { status: string }) {
  const colourMap: Record<string, string> = {
    new: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    modified: 'bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200',
    skipped: 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300',
    created: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    updated: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
    error: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
  }
  return (
    <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${colourMap[status] || ''}`}>
      {status}
    </span>
  )
}

/** Loading skeleton shown while scan is in progress. */
function ScanSkeleton() {
  return (
    <div className="space-y-2 animate-pulse p-2">
      {[1, 2, 3].map((i) => (
        <div key={i} className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-full" />
      ))}
    </div>
  )
}

// ─── Main Component ──────────────────────────────────────────────────────────

/**
 * RightSidebar section for manual Markdown file ingestion (GS-9).
 *
 * Implements INV-3 (two-step scan → confirm flow). The panel does NOT
 * auto-scan on open — the user must explicitly click "Scan for files".
 */
export const MigrationPanel = memo(function MigrationPanel() {
  const [plan, setPlan] = useState<IngestionPlan | null>(null)
  const [scanning, setScanning] = useState(false)
  const [scanError, setScanError] = useState<string | null>(null)
  const [operating, setOperating] = useState(false)
  const [result, setResult] = useState<MigrationResponse | null>(null)
  const [opError, setOpError] = useState<string | null>(null)
  const [confirmAction, setConfirmAction] = useState<'ingest' | 'reindex' | null>(null)
  const depthRef = useRef<HTMLInputElement>(null)

  // ── Scan ───────────────────────────────────────────────────────────────────

  const handleScan = useCallback(async () => {
    setScanning(true)
    setScanError(null)
    setPlan(null)
    setResult(null)
    setConfirmAction(null)

    try {
      const depth = depthRef.current ? parseInt(depthRef.current.value, 10) || DEFAULT_DEPTH : DEFAULT_DEPTH
      const data = await api.scanForImport(depth)
      setPlan(data)
    } catch (err) {
      setScanError(err instanceof Error ? err.message : 'Scan failed')
    } finally {
      setScanning(false)
    }
  }, [])

  // ── Ingest ─────────────────────────────────────────────────────────────────

  const handleIngest = useCallback(async () => {
    if (!plan) return
    setOperate(true)
    setOpError(null)
    setResult(null)
    setConfirmAction(null)

    try {
      const data = await api.ingestMd(plan)
      setResult(data)
      // Re-scan to refresh status after ingest
      handleScan()
    } catch (err) {
      setOpError(err instanceof Error ? err.message : 'Ingest failed')
    } finally {
      setOperating(false)
    }
  }, [plan, handleScan])

  // ── Reindex ────────────────────────────────────────────────────────────────

  const handleReindex = useCallback(async () => {
    if (!plan) return
    setOperate(true)
    setOpError(null)
    setResult(null)
    setConfirmAction(null)

    try {
      const data = await api.reindexMd(plan)
      setResult(data)
      // Re-scan to refresh status after reindex
      handleScan()
    } catch (err) {
      setOpError(err instanceof Error ? err.message : 'Reindex failed')
    } finally {
      setOperating(false)
    }
  }, [plan, handleScan])

  // ── Derived state ──────────────────────────────────────────────────────────

  const hasNewFiles = plan?.summary.new ?? 0 > 0
  const hasModifiedFiles = plan?.summary.modified ?? 0 > 0
  const totalCandidates = plan?.summary.total ?? 0

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="p-3 space-y-3 text-sm" data-testid="migration-panel">
      {/* Header with scan controls */}
      <div className="flex items-center gap-2">
        <h3 className="font-semibold text-sm flex-1">Import Markdown</h3>
      </div>

      {/* Depth control */}
      <div className="flex items-center gap-2">
        <label htmlFor="migration-depth" className="text-xs text-gray-500 dark:text-gray-400">
          Depth:
        </label>
        <input
          ref={depthRef}
          id="migration-depth"
          type="number"
          min={1}
          max={16}
          defaultValue={DEFAULT_DEPTH}
          className="w-14 px-1.5 py-0.5 text-xs border rounded dark:bg-gray-800 dark:border-gray-600"
        />
        <button
          onClick={handleScan}
          disabled={scanning || operating}
          className="px-3 py-1 text-xs font-medium rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
          data-testid="migration-scan-btn"
        >
          {scanning ? 'Scanning…' : 'Scan for files'}
        </button>
      </div>

      {/* Scan error */}
      {scanError && (
        <div className="p-2 text-xs text-red-700 bg-red-50 dark:bg-red-900/30 dark:text-red-300 rounded" data-testid="migration-scan-error">
          {scanError}
        </div>
      )}

      {/* Scanning skeleton */}
      {scanning && <ScanSkeleton />}

      {/* Plan results */}
      {plan && !scanning && (
        <div className="space-y-2">
          {/* Summary bar */}
          <div className="flex gap-2 text-xs" data-testid="migration-summary">
            <span className="px-2 py-0.5 rounded bg-green-100 dark:bg-green-900/50">
              {plan.summary.new} new
            </span>
            <span className="px-2 py-0.5 rounded bg-amber-100 dark:bg-amber-900/50">
              {plan.summary.modified} modified
            </span>
            <span className="px-2 py-0.5 rounded bg-gray-100 dark:bg-gray-700">
              {plan.summary.skipped} skipped
            </span>
          </div>

          {/* Candidate list */}
          {totalCandidates > 0 ? (
            <ul className="space-y-1 max-h-60 overflow-y-auto" data-testid="migration-candidates">
              {plan.candidates.map((c) => (
                <li key={c.sourcePath} className="flex items-center justify-between py-1 px-2 rounded bg-gray-50 dark:bg-gray-800 text-xs">
                  <span className="truncate flex-1 mr-2 font-mono" title={c.sourcePath}>
                    {c.sourcePath}
                  </span>
                  <StatusBadge status={c.status} />
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-xs text-gray-500 dark:text-gray-400 italic">
              No Markdown files found in the graph directory.
            </p>
          )}

          {/* Action buttons */}
          {totalCandidates > 0 && (
            <div className="flex gap-2 pt-1">
              {confirmAction ? (
                // Confirmation mode (INV-3: explicit confirm)
                <div className="flex-1 space-y-1">
                  <p className="text-xs text-amber-700 dark:text-amber-300">
                    {confirmAction === 'ingest'
                      ? `Ingest ${plan.summary.new} new file(s)?`
                      : `Reindex ${plan.summary.modified} modified file(s)?`
                    }
                  </p>
                  <div className="flex gap-2">
                    <button
                      onClick={confirmAction === 'ingest' ? handleIngest : handleReindex}
                      disabled={operating}
                      className="px-3 py-1 text-xs font-medium rounded bg-green-600 text-white hover:bg-green-700 disabled:opacity-50"
                      data-testid="migration-confirm-btn"
                    >
                      {operating ? 'Working…' : 'Confirm'}
                    </button>
                    <button
                      onClick={() => setConfirmAction(null)}
                      disabled={operating}
                      className="px-3 py-1 text-xs rounded bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 disabled:opacity-50"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                // Action selection mode
                <>
                  <button
                    onClick={() => setConfirmAction('ingest')}
                    disabled={!hasNewFiles || operating}
                    className="flex-1 px-3 py-1 text-xs font-medium rounded bg-green-600 text-white hover:bg-green-700 disabled:opacity-30"
                    data-testid="migration-ingest-btn"
                  >
                    Ingest New
                  </button>
                  <button
                    onClick={() => setConfirmAction('reindex')}
                    disabled={!hasModifiedFiles || operating}
                    className="flex-1 px-3 py-1 text-xs font-medium rounded bg-amber-600 text-white hover:bg-amber-700 disabled:opacity-30"
                    data-testid="migration-reindex-btn"
                  >
                    Reindex
                  </button>
                </>
              )}
            </div>
          )}

          {/* Operation error */}
          {opError && (
            <div className="p-2 text-xs text-red-700 bg-red-50 dark:bg-red-900/30 dark:text-red-300 rounded" data-testid="migration-op-error">
              {opError}
            </div>
          )}

          {/* Results */}
          {result && result.results.length > 0 && (
            <div className="space-y-1 pt-1 border-t dark:border-gray-700" data-testid="migration-results">
              <p className="text-xs font-medium">
                {result.totalPagesCreated} pages created, {result.totalBlocksCreated} blocks, {result.totalUpdated} updated, {result.totalSkipped} skipped
              </p>
              <ul className="space-y-0.5 max-h-40 overflow-y-auto">
                {result.results.map((r: CandidateResult) => (
                  <li key={r.sourcePath} className="flex items-center justify-between py-0.5 px-2 rounded bg-gray-50 dark:bg-gray-800 text-xs">
                    <span className="truncate flex-1 mr-2 font-mono">{r.sourcePath}</span>
                    <StatusBadge status={r.status} />
                    {r.warning && (
                      <span className="ml-1 text-amber-600 dark:text-amber-400" title={r.warning}>⚠</span>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* Empty state — no scan yet */}
      {!plan && !scanning && !scanError && (
        <p className="text-xs text-gray-500 dark:text-gray-400 italic">
          Scan your graph directory for Markdown files to import.
        </p>
      )}
    </div>
  )
})

export const MIGRATION_SECTION_ID = SECTION_ID
