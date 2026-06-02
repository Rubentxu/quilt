/**
 * Cursor/selection utilities for contentEditable elements.
 * Used by the outliner's BlockRow for cursor positioning during
 * split, merge, and focus operations.
 */

/**
 * Set cursor position inside a contentEditable element.
 * @param element The contentEditable HTMLElement
 * @param position 'start' = beginning of content, 'end' = end of content
 */
export function setCursorAt(element: HTMLElement, position: 'start' | 'end'): void {
  const range = document.createRange()
  const sel = window.getSelection()
  if (!sel) return

  if (position === 'start') {
    range.setStart(element, 0)
    range.collapse(true)
  } else {
    range.selectNodeContents(element)
    range.collapse(false)
  }

  sel.removeAllRanges()
  sel.addRange(range)
}

/**
 * Get the cursor caret offset (character index) inside a contentEditable element.
 * Returns 0 if selection is unavailable or the element is not focused.
 */
export function getCursorPosition(element: HTMLElement, mode: 'start' | 'end' = 'start'): number {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0) return 0

  const range = sel.getRangeAt(0)

  // If the cursor is outside our element, return 0
  if (!element.contains(range.startContainer)) return 0

  const preRange = range.cloneRange()
  preRange.selectNodeContents(element)
  preRange.setEnd(
    mode === 'start' ? range.startContainer : range.endContainer,
    mode === 'start' ? range.startOffset : range.endOffset,
  )
  return preRange.toString().length
}

/**
 * Check if cursor is at the very start of a contentEditable element.
 */
export function isCursorAtStart(element: HTMLElement): boolean {
  return getCursorPosition(element) === 0
}

/**
 * Check if cursor is at the very end of a contentEditable element.
 */
export function isCursorAtEnd(element: HTMLElement): boolean {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0) return false

  const range = sel.getRangeAt(0)

  // If the cursor is outside our element, treat as not-end
  if (!element.contains(range.startContainer)) return false

  const fullLen = element.textContent?.length ?? 0
  return getCursorPosition(element) >= fullLen
}
