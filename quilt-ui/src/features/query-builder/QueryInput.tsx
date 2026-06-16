/**
 * QueryInput — DSL text input with real-time validation and error display.
 *
 * Features:
 * - Text input for raw DSL queries
 * - Real-time validation via WASM query_validate (debounced 300ms)
 * - Error display with message and "Show in docs" link
 * - Optional chips-mode toggle (FilterChipGroup vs raw DSL input)
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import { AlertCircle, ExternalLink, ChevronDown, ChevronUp, ListFilter } from 'lucide-react'
import type { QueryAst } from '@shared/types/queryAst'
import { validateQuery } from '@shared/utils/validateQuery'
import { FilterChipGroup } from '../filter-chips/FilterChipGroup'
import type { FilterChip } from '@shared/types/filterChip'
import { buildQueryAst } from '@shared/utils/buildQueryAst'
import { validateChipList } from '@shared/types/filterChip'

// ─── Error shape ─────────────────────────────────────────────────

export interface QueryInputError {
  /** The raw error message from the parser. */
  message: string
  /**
   * Line number (1-based) where the error occurred, if available.
   * Currently the parser does not track position, so this is always undefined.
   * The UI still renders the field label to reserve space for future use.
   */
  line?: number
  /** Column number (1-based), if available. */
  column?: number
}

// ─── Props ──────────────────────────────────────────────────────

interface QueryInputProps {
  /** Current DSL query string. */
  value: string
  /** Called when the user types (debounced for validation). */
  onChange: (value: string) => void
  /** Called when the user presses Enter or clicks Run. */
  onExecute: (dsl: string, ast: QueryAst | null) => void
  /** Available property keys for the chip-based input mode. */
  availableKeys?: string[]
  /** Whether the input is disabled. */
  disabled?: boolean
  /** Current error, if any. */
  error?: QueryInputError | null
  /** Called when error is set/cleared. */
  onErrorChange?: (error: QueryInputError | null) => void
  /** Chips state (for chip-mode). */
  chips?: FilterChip[]
  /** Called when chips change (for chip-mode). */
  onChipsChange?: (chips: FilterChip[]) => void
  /** Called when chips are applied (for chip-mode). */
  onChipsApply?: (chips: FilterChip[]) => void
  /** Whether to start in chips mode (default: false = DSL input). */
  initialMode?: 'dsl' | 'chips'
}

// ─── Component ──────────────────────────────────────────────────

/**
 * Dual-mode query input:
 * - DSL mode: raw text input with WASM validation
 * - Chips mode: FilterChipGroup builder
 *
 * Toggle between modes via the mode button.
 * Errors are displayed inline below the input with a docs link.
 */
export function QueryInput({
  value,
  onChange,
  onExecute,
  availableKeys,
  disabled,
  error,
  onErrorChange,
  chips = [],
  onChipsChange,
  onChipsApply,
  initialMode = 'dsl',
}: QueryInputProps) {
  const [mode, setMode] = useState<'dsl' | 'chips'>(initialMode)
  const [internalError, setInternalError] = useState<QueryInputError | null>(null)
  const [expanded, setExpanded] = useState(false)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Use internal error unless error prop is provided
  const displayError = error !== undefined ? error : internalError

  function setError(err: QueryInputError | null) {
    setInternalError(err)
    onErrorChange?.(err)
  }

  // Debounced real-time validation via WASM
  const validate = useCallback(
    async (dsl: string) => {
      if (!dsl.trim()) {
        setError(null)
        return
      }
      try {
        const result = await validateQuery(dsl)
        if (result.valid) {
          setError(null)
        } else {
          setError({
            message: result.error ?? 'Invalid query',
            // Position info not available from the current parser
            line: undefined,
            column: undefined,
          })
        }
      } catch {
        // WASM not available — skip client validation, let server handle errors
        setError(null)
      }
    },
    [],
  )

  // Debounce validation on value change
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(() => {
      validate(value)
    }, 300)
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    }
  }, [value, validate])

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleRun()
    }
  }

  function handleRun() {
    if (disabled || !value.trim()) return
    // Run validation synchronously first
    validate(value).then(() => {
      if (!internalError) {
        // Deferred: parse and execute
        validateQuery(value).then(result => {
          if (result.valid && result.ast) {
            onExecute(value, result.ast)
          }
        })
      }
    })
  }

  function handleChipsApply(appliedChips: FilterChip[]) {
    const errors = validateChipList(appliedChips)
    if (Object.keys(errors).length > 0) return
    const ast = buildQueryAst(appliedChips)
    const dsl = chipsToDsl(appliedChips)
    onExecute(dsl, ast)
  }

  function toggleMode() {
    setMode(m => (m === 'dsl' ? 'chips' : 'dsl'))
  }

  return (
    <div
      data-testid="query-input"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-2)',
      }}
    >
      {/* ─── Toolbar: mode toggle + Run button ─── */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
        }}
      >
        {/* Mode toggle */}
        <button
          type="button"
          data-testid="query-input-mode-toggle"
          onClick={toggleMode}
          disabled={disabled}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '4px var(--space-2)',
            background: 'transparent',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--color-text-muted)',
            fontSize: 'var(--font-size-caption)',
            cursor: disabled ? 'not-allowed' : 'pointer',
            opacity: disabled ? 0.5 : 1,
          }}
          title={mode === 'dsl' ? 'Switch to chip builder' : 'Switch to DSL input'}
        >
          <ListFilter size={12} />
          {mode === 'dsl' ? 'Chips' : 'DSL'}
        </button>

        {/* DSL input / Chips mode */}
        <div style={{ flex: 1 }}>
          {mode === 'dsl' ? (
            <textarea
              ref={inputRef}
              data-testid="query-input-dsl"
              value={value}
              onChange={e => onChange(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={disabled}
              placeholder="(task todo) or (page &quot;My Page&quot;)"
              rows={1}
              style={{
                width: '100%',
                padding: '6px var(--space-2)',
                border: `1px solid ${displayError ? 'var(--color-destructive)' : 'var(--color-border)'}`,
                borderRadius: 'var(--radius-sm)',
                fontSize: '14px',
                fontFamily: 'var(--font-family-mono)',
                background: 'var(--color-surface)',
                color: 'var(--color-text-primary)',
                resize: 'none',
                outline: 'none',
                opacity: disabled ? 0.5 : 1,
              }}
            />
          ) : (
            <FilterChipGroup
              chips={chips}
              onChange={onChipsChange ?? (() => {})}
              availableKeys={availableKeys}
              disabled={disabled}
              onApply={handleChipsApply}
            />
          )}
        </div>

        {/* Run button (DSL mode only) */}
        {mode === 'dsl' && (
          <button
            type="button"
            data-testid="query-input-run"
            onClick={handleRun}
            disabled={disabled || !value.trim()}
            style={{
              padding: '6px var(--space-3)',
              background: disabled || !value.trim()
                ? 'var(--color-surface-subtle)'
                : 'var(--color-accent)',
              color: disabled || !value.trim()
                ? 'var(--color-text-muted)'
                : 'var(--color-surface)',
              border: 'none',
              borderRadius: 'var(--radius-sm)',
              fontSize: 'var(--font-size-caption)',
              fontWeight: 600,
              cursor: disabled || !value.trim() ? 'not-allowed' : 'pointer',
              opacity: disabled || !value.trim() ? 0.5 : 1,
              whiteSpace: 'nowrap',
            }}
          >
            Run
          </button>
        )}

        {/* Expand/collapse toggle */}
        {mode === 'dsl' && (
          <button
            type="button"
            data-testid="query-input-expand"
            onClick={() => setExpanded(e => !e)}
            disabled={disabled}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '4px',
              background: 'transparent',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--color-text-muted)',
              cursor: disabled ? 'not-allowed' : 'pointer',
              opacity: disabled ? 0.5 : 1,
            }}
            title={expanded ? 'Collapse' : 'Expand'}
          >
            {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>
        )}
      </div>

      {/* ─── Error display ─── */}
      {displayError && (
        <div
          data-testid="query-input-error"
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            gap: 'var(--space-2)',
            padding: 'var(--space-2) var(--space-3)',
            background: 'var(--color-destructive-bg, color-muted)',
            border: '1px solid var(--color-destructive)',
            borderRadius: 'var(--radius-sm)',
          }}
          role="alert"
        >
          <AlertCircle
            size={14}
            style={{
              color: 'var(--color-destructive)',
              flexShrink: 0,
              marginTop: '1px',
            }}
          />
          <div
            style={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              gap: '2px',
            }}
          >
            <span
              style={{
                fontSize: 'var(--font-size-caption)',
                color: 'var(--color-text-primary)',
                fontFamily: 'var(--font-family-mono)',
              }}
            >
              {displayError.line !== undefined
                ? `Line ${displayError.line}, Col ${displayError.column}: `
                : ''}
              {displayError.message}
            </span>
            <a
              href="https://quilt-cg4.example.com/docs/query-dsl"
              target="_blank"
              rel="noopener noreferrer"
              data-testid="query-input-error-docs-link"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '3px',
                fontSize: 'var(--font-size-micro)',
                color: 'var(--color-text-muted)',
                textDecoration: 'underline',
              }}
            >
              Show in docs
              <ExternalLink size={10} />
            </a>
          </div>
        </div>
      )}

      {/* ─── Expanded: syntax hints ─── */}
      {expanded && mode === 'dsl' && (
        <div
          data-testid="query-input-hints"
          style={{
            padding: 'var(--space-2) var(--space-3)',
            background: 'var(--color-surface-subtle)',
            borderRadius: 'var(--radius-sm)',
            fontSize: 'var(--font-size-caption)',
            color: 'var(--color-text-muted)',
            fontFamily: 'var(--font-family-mono)',
          }}
        >
          <div style={{ marginBottom: 'var(--space-1)', fontWeight: 600 }}>
            DSL Syntax
          </div>
          <div>
            <code>(task todo)</code> ·{' '}
            <code>(priority a)</code> ·{' '}
            <code>(page &quot;Name&quot;)</code> ·{' '}
            <code>(tags &quot;tag&quot;)</code>
          </div>
          <div>
            <code>(and (task todo) (priority a))</code> ·{' '}
            <code>(or ...)</code> ·{' '}
            <code>(not ...)</code>
          </div>
          <div>
            <code>(scheduled today)</code> ·{' '}
            <code>(overdue)</code> ·{' '}
            <code>(in-progress)</code>
          </div>
          <div>
            <code>(temporal :today (page &quot;x&quot;))</code> ·{' '}
            <code>(temporal :this-week ...)</code>
          </div>
        </div>
      )}
    </div>
  )
}

// ─── Chips → DSL conversion ─────────────────────────────────────

/**
 * Convert a FilterChip list back to a DSL string for display
 * in the DSL input when switching modes.
 */
function chipsToDsl(chips: FilterChip[]): string {
  const parts = chips.map(chip => {
    if (!chip.key) return ''
    if (chip.value2 !== undefined) {
      return `(property "${chip.key}" "${chip.op}" "${chip.value}" "${chip.value2}")`
    }
    return `(property "${chip.key}" "${chip.value}")`
  })
  return parts.filter(Boolean).join(' ')
}
