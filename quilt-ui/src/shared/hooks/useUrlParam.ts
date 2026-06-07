import { useState, useEffect, useCallback, useRef } from 'react'

/**
 * Read and update a single URL query parameter. Returns a tuple of
 * `[value, setter]` where:
 *
 *   - `value` is the current param value (`null` if the param is
 *     not present in the URL).
 *   - `setter` is a function that updates the URL: pass a string
 *     to set, or `null` to remove the param. Other existing query
 *     params are preserved.
 *
 * The hook listens for `popstate` events so back/forward navigation
 * and external code that mutates the URL (e.g. another component
 * calling `history.pushState`) keeps the returned value in sync.
 *
 * @example
 *   const [zoomId, setZoomId] = useUrlParam('zoom')
 *   setZoomId('block-123')          // URL → ?zoom=block-123
 *   setZoomId(null)                 // URL → (zoom= removed)
 */
export function useUrlParam(
  key: string,
): [string | null, (next: string | null) => void] {
  const [value, setValue] = useState<string | null>(() => readParam(key))

  // The setter we hand out is stable across renders so callers
  // can include it in dependency lists without retriggering
  // effects. It uses a ref to read the latest `key` argument
  // in case the hook is used with a dynamic key.
  const keyRef = useRef(key)
  keyRef.current = key

  const setParam = useCallback((next: string | null) => {
    const url = new URL(window.location.href)
    if (next == null || next === '') {
      url.searchParams.delete(keyRef.current)
    } else {
      url.searchParams.set(keyRef.current, next)
    }
    window.history.replaceState({}, '', url.toString())
    setValue(next == null || next === '' ? null : next)
  }, [])

  useEffect(() => {
    function onPopState() {
      setValue(readParam(keyRef.current))
    }
    window.addEventListener('popstate', onPopState)
    return () => window.removeEventListener('popstate', onPopState)
  }, [])

  return [value, setParam]
}

function readParam(key: string): string | null {
  if (typeof window === 'undefined') return null
  const params = new URLSearchParams(window.location.search)
  const v = params.get(key)
  // Treat empty-string params as missing for consistency with
  // the setter, which also strips them.
  return v == null || v === '' ? null : v
}
