import { useState, useEffect, useRef, useMemo } from 'react'
import {
  Command as CommandIcon,
  X,
  Compass,
  Eye,
  Pen,
  Keyboard,
  CircleHelp,
} from 'lucide-react'
import {
  CommandRegistryProvider,
  useCommandRegistry,
} from './context'
import { CommandCategory, type Command } from './types'

interface CommandCenterProps {
  isOpen: boolean
  onClose: () => void
}

// ──── Category icon map ─────────────────────────────────────────
//
// Icons are a PRESENTATION concern — the Command type stays
// JSX-free so the registry can be tested without React. The
// modal renders an icon by looking up the command's category in
// this map. Adding a new category is a one-line change here.

const CATEGORY_ICONS: Record<CommandCategory, React.ReactNode> = {
  [CommandCategory.Navigation]: <Compass size={16} style={{ flexShrink: 0, color: 'var(--color-accent)' }} />,
  [CommandCategory.View]: <Eye size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />,
  [CommandCategory.Capture]: <Pen size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />,
  [CommandCategory.Edit]: <Pen size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />,
  [CommandCategory.Help]: <CircleHelp size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />,
}

/**
 * Modal command palette.
 *
 * Triggered by Cmd/Ctrl+Shift+K from the AppShell. The component is
 * PRESENTATIONAL — it does not own any command data; it pulls
 * the list and the search/execute functions from
 * `useCommandRegistry()`. The provider is expected to be mounted
 * higher in the tree (see `main.tsx`).
 *
 * Layout: fixed overlay at z-index 100, max-width 640px, centered
 * horizontally, anchored near the top of the viewport. Mirrors
 * `SearchModal` so users get the same affordance from both
 * palettes.
 */
export function CommandCenter({ isOpen, onClose }: CommandCenterProps) {
  // We use the registry's `search` + `execute` directly. The
  // provider is mounted in `main.tsx`, so the modal is just a
  // consumer.
  return (
    <>
      {isOpen && (
        <CommandCenterInner isOpen={isOpen} onClose={onClose} />
      )}
    </>
  )
}

function CommandCenterInner({ isOpen, onClose }: CommandCenterProps) {
  const { search, execute } = useCommandRegistry()
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  // Re-derive the result list on every render. `search` is a pure
  // function of (commands, query), and React's reconciler is fast
  // enough at this scale (we cap the visible result set below) to
  // skip useMemo. If profiling later shows this is hot, wrap in
  // useMemo([query, commands]).
  const results = useMemo(() => search(query), [search, query])

  // Reset the input and the cursor every time the modal opens.
  // Without this, reopening the palette would show the previous
  // query and a stale selection — a surprising UX.
  useEffect(() => {
    if (isOpen) {
      setQuery('')
      setSelectedIndex(0)
      // Use RAF to ensure the input is mounted before focusing.
      const raf = requestAnimationFrame(() => inputRef.current?.focus())
      return () => cancelAnimationFrame(raf)
    }
  }, [isOpen])

  // Clamp the cursor when the result list shrinks (e.g. user types
  // a query that filters out the row that was selected).
  useEffect(() => {
    if (selectedIndex >= results.length) {
      setSelectedIndex(Math.max(0, results.length - 1))
    }
  }, [results.length, selectedIndex])

  function runSelected() {
    const cmd = results[selectedIndex]
    if (!cmd) return
    // Pass the typed query so context-aware commands (Quick
    // Capture, etc.) can use it.
    execute(cmd.id, { query: query.trim() || undefined })
    onClose()
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex((i) => Math.min(i + 1, Math.max(0, results.length - 1)))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      runSelected()
    } else if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
    }
  }

  return (
    <div
      data-testid="command-center-backdrop"
      role="presentation"
      onClick={onClose}
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 100,
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        paddingTop: '15vh',
        background: 'rgba(0, 0, 0, 0.4)',
      }}
    >
      <div
        role="dialog"
        aria-label="Command palette"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
        style={{
          width: '100%',
          maxWidth: '640px',
          background: 'var(--color-surface)',
          borderRadius: 'var(--radius-lg)',
          boxShadow: 'var(--shadow-lg)',
          overflow: 'hidden',
        }}
      >
        {/* ─── Input row ─── */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: 'var(--space-3) var(--space-4)',
            borderBottom: '1px solid var(--color-border)',
            gap: 'var(--space-3)',
          }}
        >
          <CommandIcon
            size={18}
            style={{ color: 'var(--color-text-muted)', flexShrink: 0 }}
          />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value)
              setSelectedIndex(0)
            }}
            onKeyDown={handleKeyDown}
            placeholder="Type a command…"
            aria-label="Command palette search"
            aria-controls="command-center-listbox"
            aria-activedescendant={
              results[selectedIndex] ? `command-row-${results[selectedIndex].id}` : undefined
            }
            style={{
              flex: 1,
              border: 'none',
              outline: 'none',
              fontSize: '16px',
              background: 'transparent',
              color: 'var(--color-text-primary)',
            }}
          />
          <button
            onClick={onClose}
            aria-label="Close command palette"
            data-testid="command-center-close"
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              display: 'inline-flex',
              alignItems: 'center',
              padding: '2px',
            }}
          >
            <X size={14} />
          </button>
        </div>

        {/* ─── Results ─── */}
        <div
          id="command-center-listbox"
          role="listbox"
          aria-label="Commands"
          style={{ maxHeight: '400px', overflowY: 'auto' }}
        >
          {results.length === 0 && (
            <div
              data-testid="command-center-empty"
              style={{
                padding: 'var(--space-8)',
                textAlign: 'center',
                color: 'var(--color-text-muted)',
                fontSize: '14px',
              }}
            >
              {query.trim()
                ? 'No commands match your search.'
                : 'No commands available.'}
            </div>
          )}

          {results.map((cmd, idx) => (
            <CommandRow
              key={cmd.id}
              command={cmd}
              selected={idx === selectedIndex}
              onSelect={() => {
                setSelectedIndex(idx)
                // Defer to next tick so React renders the
                // selection highlight before we close.
                setTimeout(() => {
                  execute(cmd.id, { query: query.trim() || undefined })
                  onClose()
                }, 0)
              }}
              onHover={() => setSelectedIndex(idx)}
            />
          ))}
        </div>

        {/* ─── Footer hint ─── */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-3)',
            padding: 'var(--space-2) var(--space-4)',
            borderTop: '1px solid var(--color-border)',
            background: 'var(--color-surface-subtle)',
            fontSize: 'var(--font-size-caption)',
            color: 'var(--color-text-muted)',
          }}
        >
          <Keyboard size={12} />
          <span>
            <kbd style={kbdStyle}>↑↓</kbd> navigate
            {'  '}
            <kbd style={kbdStyle}>↵</kbd> select
            {'  '}
            <kbd style={kbdStyle}>Esc</kbd> close
          </span>
        </div>
      </div>
    </div>
  )
}

const kbdStyle: React.CSSProperties = {
  fontSize: '10px',
  fontFamily: 'inherit',
  padding: '1px 4px',
  background: 'var(--color-surface)',
  border: '1px solid var(--color-border)',
  borderRadius: '3px',
  color: 'var(--color-text-secondary)',
}

// ──── Single result row ────────────────────────────────────────

interface CommandRowProps {
  command: Command
  selected: boolean
  onSelect: () => void
  onHover: () => void
}

function CommandRow({ command, selected, onSelect, onHover }: CommandRowProps) {
  return (
    <button
      id={`command-row-${command.id}`}
      role="option"
      aria-selected={selected}
      data-testid={`command-row-${command.id}`}
      onClick={onSelect}
      onMouseEnter={onHover}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-3)',
        width: '100%',
        padding: 'var(--space-3) var(--space-4)',
        border: 'none',
        cursor: 'pointer',
        textAlign: 'left',
        background: selected ? 'var(--color-surface-subtle)' : 'transparent',
        color: 'var(--color-text-primary)',
        fontSize: '14px',
        fontFamily: 'inherit',
        transition: 'background var(--motion-fast) var(--ease-standard)',
      }}
    >
      {CATEGORY_ICONS[command.category as CommandCategory] ??
        CATEGORY_ICONS[CommandCategory.Edit]}
      <span
        style={{
          flex: 1,
          minWidth: 0,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
      >
        {command.label}
      </span>
      <CategoryBadge category={command.category} />
      {command.shortcut && (
        <span
          style={{
            fontSize: 'var(--font-size-micro)',
            fontFamily: 'var(--font-family-mono, monospace)',
            fontWeight: 500,
            color: 'var(--color-text-muted)',
            background: 'var(--color-surface-subtle)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-sm)',
            padding: '1px var(--space-2)',
            flexShrink: 0,
          }}
        >
          {command.shortcut}
        </span>
      )}
    </button>
  )
}

function CategoryBadge({ category }: { category: CommandCategory }) {
  // The category string itself doubles as the badge label. We do
  // NOT translate here — the Command type carries the canonical
  // English label and i18n is a future concern (a CategoryBadge
  // lookup table would replace this).
  return (
    <span
      style={{
        fontSize: 'var(--font-size-micro)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-wider)',
        color: 'var(--color-text-muted)',
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-pill)',
        padding: '2px var(--space-2)',
        flexShrink: 0,
      }}
    >
      {category}
    </span>
  )
}

// Re-export the provider so feature-local imports stay terse:
// `import { CommandCenter, CommandRegistryProvider } from
// '@features/command-center'`. Avoids a separate import for the
// most common wiring sites.
export { CommandRegistryProvider }
