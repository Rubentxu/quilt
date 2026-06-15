// ──── useDatePickerPopover — quilt-slash-command-functional-behaviour ────
//
// Manages the DatePicker popover lifecycle:
//   - Anchoring to a trigger element
//   - Keyboard handling (Escape to cancel)
//   - Click-outside to cancel
//
// This hook is intentionally minimal — it owns the open/close state
// and the outside-click detection. The actual DatePicker rendering
// is handled by the DatePicker component itself.

import { useCallback, useEffect, useRef, useState } from 'react'

export interface UseDatePickerPopoverOptions {
  /** Called when the user commits a date (click, Enter on NL input). */
  onSelect: (iso: string) => void
  /** Called when the user cancels (Escape, click-outside). */
  onCancel?: () => void
  /** Whether the popover is currently open. */
  open: boolean
  /** Callback to close the popover. */
  onClose: () => void
  /** Optional ref to the trigger element (for positioning). */
  triggerRef?: React.RefObject<HTMLElement | null>
}

/**
 * Returns positioning styles for the popover anchored below the trigger.
 * Uses getBoundingClientRect for accurate placement.
 */
function getPopoverStyle(triggerEl: HTMLElement): React.CSSProperties {
  const rect = triggerEl.getBoundingClientRect()
  const scrollX = window.scrollX
  const scrollY = window.scrollY
  return {
    position: 'absolute',
    top: rect.bottom + scrollY + 4,
    left: rect.left + scrollX,
    zIndex: 9999,
  }
}

export function useDatePickerPopover({
  onSelect,
  onCancel,
  open,
  onClose,
  triggerRef,
}: UseDatePickerPopoverOptions) {
  const popoverRef = useRef<HTMLDivElement>(null)
  const [popoverStyle, setPopoverStyle] = useState<React.CSSProperties>({
    position: 'absolute',
    zIndex: 9999,
  })

  // Update popover position when it opens
  useEffect(() => {
    if (!open || !triggerRef?.current) return
    setPopoverStyle(getPopoverStyle(triggerRef.current))
  }, [open, triggerRef])

  // Outside-click detection
  useEffect(() => {
    if (!open) return

    function handleClickOutside(e: MouseEvent) {
      const target = e.target as Node
      if (
        popoverRef.current &&
        !popoverRef.current.contains(target) &&
        triggerRef?.current &&
        !triggerRef.current.contains(target)
      ) {
        e.preventDefault()
        e.stopPropagation()
        onCancel?.()
        onClose()
      }
    }

    // Use setTimeout to avoid the opening click itself being caught
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside, true)
    }, 0)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('mousedown', handleClickOutside, true)
    }
  }, [open, onCancel, onClose, triggerRef])

  /** Wrapper around onSelect that also closes the popover. */
  const handleSelect = useCallback(
    (iso: string) => {
      onSelect(iso)
      onClose()
    },
    [onSelect, onClose],
  )

  return {
    popoverRef,
    popoverStyle,
    handleSelect,
    handleCancel: onCancel,
  }
}
