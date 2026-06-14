/**
 * Tests for the `featureFlags` module.
 *
 * The default value is true (new annotation API). The flag can be
 * overridden via the `VITE_QUILT_ANNOTATIONS_ENABLED` env var; the
 * truthy/falsy parsing mirrors the standard convention used in
 * 12-factor configs.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  parseAnnotationFlag,
  isAnnotationsEnabled,
  __setAnnotationsEnabledForTest,
} from '../featureFlags'

describe('parseAnnotationFlag', () => {
  it('returns true for undefined (default opt-in)', () => {
    expect(parseAnnotationFlag(undefined)).toBe(true)
  })

  it('returns true for empty string (default opt-in)', () => {
    expect(parseAnnotationFlag('')).toBe(true)
  })

  it('returns true for "true" / "1" / "yes" / "on" (case-insensitive)', () => {
    expect(parseAnnotationFlag('true')).toBe(true)
    expect(parseAnnotationFlag('TRUE')).toBe(true)
    expect(parseAnnotationFlag('1')).toBe(true)
    expect(parseAnnotationFlag('yes')).toBe(true)
    expect(parseAnnotationFlag('YES')).toBe(true)
    expect(parseAnnotationFlag('on')).toBe(true)
    expect(parseAnnotationFlag('On')).toBe(true)
  })

  it('returns false for "false" / "0" / "no" / "off"', () => {
    expect(parseAnnotationFlag('false')).toBe(false)
    expect(parseAnnotationFlag('FALSE')).toBe(false)
    expect(parseAnnotationFlag('0')).toBe(false)
    expect(parseAnnotationFlag('no')).toBe(false)
    expect(parseAnnotationFlag('off')).toBe(false)
  })

  it('trims surrounding whitespace before parsing', () => {
    expect(parseAnnotationFlag('  true  ')).toBe(true)
    expect(parseAnnotationFlag('  false  ')).toBe(false)
  })

  it('treats unknown strings as truthy (dev-friendly default)', () => {
    expect(parseAnnotationFlag('maybe')).toBe(true)
  })
})

describe('isAnnotationsEnabled', () => {
  let restore: () => void
  beforeEach(() => {
    restore = __setAnnotationsEnabledForTest(true)
  })
  afterEach(() => {
    restore()
  })

  it('reflects the current in-memory flag value', () => {
    expect(isAnnotationsEnabled()).toBe(true)
    const restore2 = __setAnnotationsEnabledForTest(false)
    expect(isAnnotationsEnabled()).toBe(false)
    restore2()
    expect(isAnnotationsEnabled()).toBe(true)
  })
})
