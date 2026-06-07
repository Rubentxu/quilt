// GraphViewPage — dark mode detection (F3 of P0 frontend fixes).
//
// The graph view used to read `document.documentElement.classList.contains('dark')`
// to pick light/dark canvas colors. The app actually applies the
// theme via the `data-theme` attribute on `<html>` (see
// `src/main.tsx` initialisation and `AppShell.tsx` toggle). As a
// result, switching to dark mode left the graph painted with light
// colors.
//
// The detection is extracted into a tiny pure helper so it can be
// unit-tested without spinning up a canvas / animation frame in jsdom.
// We test the BEHAVIOR — given the DOM's `data-theme` attribute, what
// does the graph think the active theme is — not the implementation
// of which DOM API it pokes at.

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { isDarkTheme } from '../GraphViewPage'

beforeEach(() => {
  // Start each test from a clean slate — the production app may have
  // left a `data-theme` attribute on the document element.
  document.documentElement.removeAttribute('data-theme')
  document.documentElement.classList.remove('dark')
})

afterEach(() => {
  document.documentElement.removeAttribute('data-theme')
  document.documentElement.classList.remove('dark')
})

describe('isDarkTheme — graph view dark mode detection', () => {
  it('returns true when <html data-theme="dark">', () => {
    document.documentElement.setAttribute('data-theme', 'dark')
    expect(isDarkTheme()).toBe(true)
  })

  it('returns false when <html data-theme="light">', () => {
    document.documentElement.setAttribute('data-theme', 'light')
    expect(isDarkTheme()).toBe(false)
  })

  it('returns false when no data-theme attribute is set (light by default)', () => {
    // No attribute — the app's default theme is light.
    expect(isDarkTheme()).toBe(false)
  })

  it('reacts to runtime attribute changes (the user can toggle theme)', () => {
    // Start light
    document.documentElement.setAttribute('data-theme', 'light')
    expect(isDarkTheme()).toBe(false)

    // Flip to dark
    document.documentElement.setAttribute('data-theme', 'dark')
    expect(isDarkTheme()).toBe(true)

    // Back to light
    document.documentElement.setAttribute('data-theme', 'light')
    expect(isDarkTheme()).toBe(false)
  })
})
