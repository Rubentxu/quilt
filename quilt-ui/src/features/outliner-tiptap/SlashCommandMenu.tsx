import { useState, useEffect, useRef } from 'react'
import { defaultRegistry, type SlashMenuItem } from './slashRegistry'

// Re-export the SlashMenuItem type so existing type-only imports
// (`import type { SlashMenuItem } from './SlashCommandMenu'`) keep
// working. The type itself lives in `slashRegistry.tsx` because the
// registry is the single source of truth for slash actions.
export type { SlashMenuItem }

/** @deprecated The legacy constant is now derived from the default
 *  registry. New code should import `defaultRegistry` from
 *  `./slashRegistry` and call `defaultRegistry.allItems()`. Kept
 *  exported because:
 *    - `SlashTemplateFlow.test.tsx` imports it to assert the
 *      "New from Template" label.
 *    - Any third-party code that imported it as a stable public
 *      surface of the SlashCommandMenu module.
 *  The constant is recomputed at module load and stays in lock-step
 *  with `defaultRegistry.allItems()` (asserted by
 *  `slashRegistry.test.ts` T10).
 */
export const SLASH_MENU_ITEMS: SlashMenuItem[] = defaultRegistry.allItems()

interface SlashCommandMenuProps {
  position: { top: number; left: number } | null
  query: string
  onSelect: (item: SlashMenuItem) => void
  onClose: () => void
}

export function SlashCommandMenu({ position, query, onSelect, onClose }: SlashCommandMenuProps) {
  const [selectedIndex, setSelectedIndex] = useState(0)
  const menuRef = useRef<HTMLDivElement>(null)

  // Read the live list from the registry so plugin authors can
  // register new items at runtime and have them appear in the menu.
  const items = defaultRegistry.allItems()

  const filteredItems = items.filter(item => {
    const q = query.toLowerCase()
    return (
      item.label.toLowerCase().includes(q) ||
      (item.blockType?.toLowerCase().includes(q) ?? false) ||
      item.keywords.some(k => k.includes(q))
    )
  })

  useEffect(() => {
    setSelectedIndex(0)
  }, [query])

  // Keyboard navigation
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (!position) return

      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          setSelectedIndex(i => (i + 1) % filteredItems.length)
          break
        case 'ArrowUp':
          e.preventDefault()
          setSelectedIndex(i => (i - 1 + filteredItems.length) % filteredItems.length)
          break
        case 'Enter':
          e.preventDefault()
          if (filteredItems[selectedIndex]) {
            onSelect(filteredItems[selectedIndex])
          }
          break
        case 'Escape':
          e.preventDefault()
          onClose()
          break
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [position, filteredItems, selectedIndex, onSelect, onClose])

  // Click outside closes menu
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [onClose])

  if (!position || filteredItems.length === 0) return null

  // Group filtered items by category
  const grouped = new Map<string, SlashMenuItem[]>()
  for (const item of filteredItems) {
    const cat = item.category
    if (!grouped.has(cat)) grouped.set(cat, [])
    grouped.get(cat)!.push(item)
  }

  return (
    <div
      ref={menuRef}
      style={{
        position: 'fixed',
        top: position.top,
        left: position.left,
        zIndex: 200,
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-lg)',
        minWidth: '280px',
        maxHeight: '420px',
        overflowY: 'auto',
        padding: 'var(--space-1) 0',
      }}
    >
      {[...grouped.entries()].map(([category, items]) => (
        <div key={category}>
          <div style={{
            padding: 'var(--space-1) var(--space-3)',
            fontSize: '11px',
            fontWeight: 600,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            background: 'var(--color-surface-subtle)',
          }}>
            {category}
          </div>
          {filteredItems.map(item => {
            const globalIdx = filteredItems.indexOf(item)
            return (
              <div
                key={item.id}
                onClick={() => onSelect(item)}
                onMouseEnter={() => setSelectedIndex(globalIdx)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-3)',
                  padding: 'var(--space-2) var(--space-3)',
                  cursor: 'pointer',
                  background: globalIdx === selectedIndex ? 'var(--color-surface-subtle)' : 'transparent',
                  transition: 'background var(--motion-fast)',
                }}
              >
                <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: '32px',
                  height: '32px',
                  borderRadius: 'var(--radius-sm)',
                  border: '1px solid var(--color-border)',
                  background: 'var(--color-surface)',
                  color: 'var(--color-text-secondary)',
                  flexShrink: 0,
                }}>
                  {item.icon}
                </div>
                <div>
                  <div style={{ fontSize: '14px', fontWeight: 500, color: 'var(--color-text-primary)' }}>
                    {item.label}
                  </div>
                  <div style={{ fontSize: '12px', color: 'var(--color-text-muted)' }}>
                    {item.description}
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      ))}
    </div>
  )
}
