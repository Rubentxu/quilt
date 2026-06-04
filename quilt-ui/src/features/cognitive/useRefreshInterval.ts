/**
 * useRefreshInterval — G7 Dream Cycle Display
 *
 * Hook for auto-refreshing data at a specified interval.
 * Cleans up the interval on unmount.
 */

import { useEffect, useRef } from 'react'

/**
 * Sets up an interval that calls the callback function at the specified interval.
 * The interval is automatically cleared when the component unmounts or when
 * the callback or interval changes.
 *
 * @param callback - Function to call on each interval
 * @param intervalMs - Interval in milliseconds. 0 disables the interval.
 */
export function useRefreshInterval(callback: () => void, intervalMs: number): void {
  const savedCallback = useRef(callback)

  // Remember the latest callback
  useEffect(() => {
    savedCallback.current = callback
  }, [callback])

  // Set up the interval
  useEffect(() => {
    if (intervalMs <= 0) {
      return
    }

    const tick = () => {
      savedCallback.current()
    }

    const id = setInterval(tick, intervalMs)
    return () => clearInterval(id)
  }, [intervalMs])
}
