// ─── useKeyboardShortcuts — global keyboard shortcut handler ─────
//
// Registers a keydown listener that fires callbacks when keyboard
// shortcuts are pressed. Supports Cmd/Ctrl + key combos and
// standalone keys. Callbacks are keyed by shortcut string:
//   - "Cmd+."   — Cmd/Ctrl + period
//   - "F11"     — function keys
//   - "Escape"  — standalone Escape
//
// The hook auto-cleans the listener on unmount.

import { useEffect } from 'react'

export interface ShortcutHandlers {
  [shortcut: string]: (e: KeyboardEvent) => void
}

/** Parse a shortcut string into a normalised representation. */
function parseShortcut(shortcut: string): { key: string; ctrl: boolean; meta: boolean; shift: boolean } {
  const parts = shortcut.toLowerCase().split('+')
  const key = parts[parts.length - 1]
  return {
    key,
    ctrl: parts.includes('ctrl'),
    meta: parts.includes('cmd') || parts.includes('meta'),
    shift: parts.includes('shift'),
  }
}

/** Check if an event matches a shortcut definition. */
function matchesShortcut(e: KeyboardEvent, shortcut: { key: string; ctrl: boolean; meta: boolean; shift: boolean }): boolean {
  const key = e.key.toLowerCase()
  if (key !== shortcut.key) return false
  const metaMatch = shortcut.meta ? e.metaKey || e.ctrlKey : true
  const ctrlMatch = shortcut.ctrl ? e.ctrlKey : true
  const shiftMatch = shortcut.shift ? e.shiftKey : true
  // For Cmd+key, allow Ctrl+key on non-Mac platforms (e.g. Linux, Windows)
  const crossPlatform = (e.metaKey || e.ctrlKey) && shortcut.meta
  return (
    key === shortcut.key &&
    (crossPlatform || metaMatch) &&
    ctrlMatch &&
    shiftMatch
  )
}

/**
 * Register global keyboard shortcut handlers.
 *
 * @param handlers — Map of shortcut string → callback.
 *                   Shortcuts are matched case-insensitively.
 *                   "Cmd+." fires on Cmd/Ctrl + period.
 *                   "Escape" fires on Escape alone.
 *
 * @example
 * useKeyboardShortcuts({
 *   'Cmd+.': () => setFocusMode(v => !v),
 *   'F11': () => setFocusMode(false),
 *   'Escape': () => setFocusMode(false),
 * })
 */
export function useKeyboardShortcuts(handlers: ShortcutHandlers): void {
  useEffect(() => {
    const parsed = Object.entries(handlers).map(([shortcut, cb]) => ({
      shortcut: parseShortcut(shortcut),
      callback: cb,
    }))

    function onKeyDown(e: KeyboardEvent) {
      // Don't fire when typing in an input/textarea/contenteditable
      const target = e.target as HTMLElement
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        return
      }

      for (const { shortcut, callback } of parsed) {
        if (matchesShortcut(e, shortcut)) {
          e.preventDefault()
          callback(e)
          return
        }
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [handlers])
}
