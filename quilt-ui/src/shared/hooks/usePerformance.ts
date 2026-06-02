import { useEffect, useRef } from 'react'

/**
 * Measure how long a component stays mounted and warn if the mount
 * lifecycle is unexpectedly long. Useful for spotting regressions
 * where a "fast" page suddenly takes 300ms+ to commit.
 *
 * Threshold is intentionally low (16ms = one frame). Anything above
 * one frame is worth a second look.
 */
export function usePerformance(label: string, warnThresholdMs = 16) {
  const startRef = useRef<number>(0)

  useEffect(() => {
    startRef.current = performance.now()
    return () => {
      const duration = performance.now() - startRef.current
      if (duration > warnThresholdMs) {
        // Surface to the console in dev — Vite strips console.log in
        // production via the esbuild drop_console option.
        // eslint-disable-next-line no-console
        console.warn(
          `[perf] ${label}: ${duration.toFixed(2)}ms (mount→unmount)`,
        )
      }
    }
  }, [label, warnThresholdMs])
}

/**
 * Wrap an async function with timing. Logs the elapsed time when the
 * promise resolves (or rejects). Useful for tracing slow IPC or WASM
 * calls without littering the codebase with `performance.now()`.
 *
 * ```ts
 * const result = await measure('parseInline', () => wasmParseInline(s))
 * ```
 */
export async function measure<T>(
  label: string,
  fn: () => Promise<T>,
  warnThresholdMs = 100,
): Promise<T> {
  const start = performance.now()
  try {
    return await fn()
  } finally {
    const duration = performance.now() - start
    if (duration > warnThresholdMs) {
      // eslint-disable-next-line no-console
      console.warn(`[perf] ${label}: ${duration.toFixed(2)}ms`)
    }
  }
}
