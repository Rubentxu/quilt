/**
 * BlockContextMenu — DESIGN.md §11.3
 *
 * Menú contextual de bloque con acciones mínimas:
 *   - Crear bloque hijo
 *   - Mover arriba
 *   - Mover abajo
 *   - Convertir en tarea
 *   - Copiar enlace al bloque
 *   - Eliminar
 *
 * Accesibilidad:
 *   - role="menu" con role="menuitem" en cada acción
 *   - Cierra con Escape
 *   - Cierra al hacer click fuera
 *   - aria-haspopup/aria-expanded en el trigger
 */

import { useEffect, useRef, type CSSProperties } from 'react'
import { Plus, ArrowUp, ArrowDown, CheckSquare, Link2, Trash2 } from 'lucide-react'

export interface BlockContextMenuActions {
  onAddChild: () => void
  onMoveUp: () => void
  onMoveDown: () => void
  onConvertToTask: () => void
  onCopyLink: () => void
  onDelete: () => void
}

interface BlockContextMenuProps {
  open: boolean
  anchorEl: HTMLElement | null
  onClose: () => void
  actions: BlockContextMenuActions
}

interface MenuItemDef {
  key: string
  label: string
  icon: React.ReactNode
  onClick: () => void
  destructive?: boolean
}

const itemBaseStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--space-2)',
  width: '100%',
  padding: 'var(--space-2) var(--space-3)',
  border: 'none',
  background: 'transparent',
  color: 'var(--color-text-primary)',
  fontSize: '13px',
  fontWeight: 400,
  textAlign: 'left',
  cursor: 'pointer',
  borderRadius: 'var(--radius-sm)',
  fontFamily: 'inherit',
  lineHeight: 1.2,
  whiteSpace: 'nowrap',
}

export function BlockContextMenu({ open, anchorEl, onClose, actions }: BlockContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null)

  // Close on outside click + Escape
  useEffect(() => {
    if (!open) return
    function handleClickOutside(e: MouseEvent) {
      const target = e.target as Node
      if (
        menuRef.current &&
        !menuRef.current.contains(target) &&
        anchorEl &&
        !anchorEl.contains(target)
      ) {
        onClose()
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.stopPropagation()
        onClose()
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleKey)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleKey)
    }
  }, [open, onClose, anchorEl])

  if (!open || !anchorEl) return null

  // Position the menu below the trigger, flipping if it would overflow.
  const rect = anchorEl.getBoundingClientRect()
  const top = rect.bottom + 4
  const left = rect.right - 220
  const safeTop = top + 220 > window.innerHeight ? rect.top - 220 - 4 : top
  const safeLeft = Math.max(8, Math.min(left, window.innerWidth - 228))

  const items: MenuItemDef[] = [
    {
      key: 'add-child',
      label: 'Add child block',
      icon: <Plus size={14} aria-hidden="true" />,
      onClick: () => {
        actions.onAddChild()
        onClose()
      },
    },
    {
      key: 'move-up',
      label: 'Move up',
      icon: <ArrowUp size={14} aria-hidden="true" />,
      onClick: () => {
        actions.onMoveUp()
        onClose()
      },
    },
    {
      key: 'move-down',
      label: 'Move down',
      icon: <ArrowDown size={14} aria-hidden="true" />,
      onClick: () => {
        actions.onMoveDown()
        onClose()
      },
    },
    {
      key: 'convert-task',
      label: 'Convert to task',
      icon: <CheckSquare size={14} aria-hidden="true" />,
      onClick: () => {
        actions.onConvertToTask()
        onClose()
      },
    },
    {
      key: 'copy-link',
      label: 'Copy block link',
      icon: <Link2 size={14} aria-hidden="true" />,
      onClick: () => {
        actions.onCopyLink()
        onClose()
      },
    },
    {
      key: 'delete',
      label: 'Delete block',
      icon: <Trash2 size={14} aria-hidden="true" />,
      onClick: () => {
        actions.onDelete()
        onClose()
      },
      destructive: true,
    },
  ]

  return (
    <div
      ref={menuRef}
      role="menu"
      aria-label="Block actions"
      data-testid="block-context-menu"
      style={{
        position: 'fixed',
        top: `${safeTop}px`,
        left: `${safeLeft}px`,
        minWidth: '200px',
        background: 'var(--color-surface-elevated)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-md)',
        padding: 'var(--space-1)',
        zIndex: 200,
        display: 'flex',
        flexDirection: 'column',
        gap: '2px',
      }}
    >
      {items.map((item) => {
        const style: CSSProperties = item.destructive
          ? { ...itemBaseStyle, color: 'var(--color-danger)' }
          : itemBaseStyle
        return (
          <button
            key={item.key}
            role="menuitem"
            onClick={item.onClick}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = item.destructive
                ? 'var(--color-danger-subtle)'
                : 'var(--color-surface-subtle)'
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent'
            }}
            style={style}
          >
            {item.icon}
            <span>{item.label}</span>
          </button>
        )
      })}
    </div>
  )
}
