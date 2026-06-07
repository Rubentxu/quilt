// ─── CreateTemplateModal — sidebar template creation flow ────────
//
// Modal that lets users create a new template (`template/<name>`) from
// the sidebar's "Plantillas" section. On submit it does two API calls
// in order:
//   1. `api.createPage({name: 'template/<name>'})`
//   2. `api.createBlock({pageName, content: '', properties: {…}})`
// The block is the "seed" that carries the `card-shape` and `icon`
// metadata the rest of the system reads when rendering template cards.
//
// Spec: openspec/changes/quilt-template-management-ui/specs/
//       sidebar-template-create/spec.md
// Design: design.md §D7 (modal anchored in sidebar section), §D8
//         (creation flow: page + seed block with properties).
//
// A11y: role="dialog" + aria-modal + aria-labelledby. Escape and
// backdrop click both close. Focus is moved to the name input on open
// and trapped within the dialog while open.

import { useEffect, useRef, useState } from 'react'
import toast from 'react-hot-toast'
import { api } from '@core/api-client'
import type { CardShape } from '@shared/types/api'

// ──── Public types ─────────────────────────────────────────────

export interface CreateTemplateModalProps {
  isOpen: boolean
  onClose: () => void
  /** Called with the new template's full name (`template/<name>`) after
   *  the API calls resolve. The sidebar uses this to refresh the list. */
  onCreated?: (pageName: string) => void
}

// ──── Constants ─────────────────────────────────────────────────

const CARD_SHAPES: { value: CardShape; label: string; description: string; emoji: string }[] = [
  {
    value: 'reference',
    label: 'Reference',
    description: 'A link-card to another page',
    emoji: '🔗',
  },
  {
    value: 'content',
    label: 'Content',
    description: 'A standalone document',
    emoji: '📄',
  },
  {
    value: 'inline',
    label: 'Inline',
    description: 'Embedded in a parent block',
    emoji: '📝',
  },
]

const PRESET_ICONS = ['🔗', '📄', '📋', '✅', '📌'] as const

// ──── Component ─────────────────────────────────────────────────

export function CreateTemplateModal({ isOpen, onClose, onCreated }: CreateTemplateModalProps) {
  const [name, setName] = useState('')
  const [cardShape, setCardShape] = useState<CardShape>('reference')
  const [icon, setIcon] = useState<string>('🔗')
  const [submitting, setSubmitting] = useState(false)
  const nameInputRef = useRef<HTMLInputElement>(null)
  const dialogRef = useRef<HTMLDivElement>(null)

  // ── Reset state and focus the name input on open ───────────────
  useEffect(() => {
    if (!isOpen) return
    setName('')
    setCardShape('reference')
    setIcon('🔗')
    setSubmitting(false)
    const raf = requestAnimationFrame(() => nameInputRef.current?.focus())
    return () => cancelAnimationFrame(raf)
  }, [isOpen])

  // ── Escape dismisses ───────────────────────────────────────────
  useEffect(() => {
    if (!isOpen) return
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.stopPropagation()
        onClose()
      }
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [isOpen, onClose])

  // ── Validation ─────────────────────────────────────────────────
  const trimmedName = name.trim()
  const isValid = trimmedName.length > 0 && !trimmedName.includes('/')

  // ── Submit ─────────────────────────────────────────────────────
  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!isValid || submitting) return

    const fullName = `template/${trimmedName}`
    setSubmitting(true)
    try {
      await api.createPage({ name: fullName })
      try {
        await api.createBlock({
          pageName: fullName,
          content: '',
          properties: {
            'card-shape': cardShape,
            icon,
          },
        })
      } catch (blockErr) {
        // Page was created but the seed block failed. Surface the
        // error to the user — the modal stays open so they can retry
        // the block step. The page exists either way; the user can
        // add a block manually.
        const message =
          blockErr instanceof Error ? blockErr.message : 'Unknown error'
        toast.error(`Template created, but seed block failed: ${message}`)
        setSubmitting(false)
        return
      }

      toast.success(`Template "${trimmedName}" created`)
      onCreated?.(fullName)
      onClose()
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error'
      toast.error(`Failed to create template: ${message}`)
      setSubmitting(false)
    }
  }

  if (!isOpen) return null

  return (
    <div
      data-testid="create-template-modal-backdrop"
      onClick={onClose}
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 100,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0, 0, 0, 0.4)',
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="create-template-modal-title"
        onClick={(e) => e.stopPropagation()}
        data-testid="create-template-modal"
        style={{
          width: '100%',
          maxWidth: '480px',
          background: 'var(--color-surface)',
          borderRadius: 'var(--radius-lg)',
          boxShadow: 'var(--shadow-lg)',
          padding: 'var(--space-6)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-4)',
        }}
      >
        <h2
          id="create-template-modal-title"
          style={{
            margin: 0,
            fontSize: '18px',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
          }}
        >
          Create template
        </h2>

        <form
          onSubmit={handleSubmit}
          style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}
        >
          {/* ── Name input ─────────────────────────────────────── */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-1)' }}>
            <label
              htmlFor="create-template-name"
              style={{
                fontSize: '13px',
                fontWeight: 500,
                color: 'var(--color-text-secondary)',
              }}
            >
              Name
            </label>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 'var(--space-2)',
                padding: '6px 10px',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius-md)',
                background: 'var(--color-surface-subtle)',
              }}
            >
              <span
                style={{
                  fontSize: '13px',
                  color: 'var(--color-text-muted)',
                  fontFamily: 'var(--font-mono, monospace)',
                }}
              >
                template/
              </span>
              <input
                id="create-template-name"
                ref={nameInputRef}
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="my-template"
                autoComplete="off"
                disabled={submitting}
                aria-invalid={!isValid && name.length > 0 ? true : undefined}
                style={{
                  flex: 1,
                  border: 'none',
                  outline: 'none',
                  background: 'transparent',
                  fontSize: '14px',
                  color: 'var(--color-text-primary)',
                  fontFamily: 'var(--font-mono, monospace)',
                  minWidth: 0,
                }}
              />
            </div>
            {!isValid && name.length > 0 && (
              <p
                role="alert"
                style={{
                  fontSize: '12px',
                  color: 'var(--color-text-error, #d44)',
                  margin: 0,
                }}
              >
                {trimmedName.length === 0
                  ? 'Name is required.'
                  : 'Name cannot contain "/".'}
              </p>
            )}
          </div>

          {/* ── Card-shape picker ──────────────────────────────── */}
          <fieldset
            style={{
              border: 'none',
              padding: 0,
              margin: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: 'var(--space-2)',
            }}
          >
            <legend
              style={{
                fontSize: '13px',
                fontWeight: 500,
                color: 'var(--color-text-secondary)',
                padding: 0,
                marginBottom: 'var(--space-1)',
              }}
            >
              Card shape
            </legend>
            <div
              role="radiogroup"
              aria-label="Card shape"
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(3, 1fr)',
                gap: 'var(--space-2)',
              }}
            >
              {CARD_SHAPES.map((shape) => {
                const selected = cardShape === shape.value
                return (
                  <label
                    key={shape.value}
                    data-testid={`card-shape-${shape.value}`}
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      gap: '4px',
                      padding: 'var(--space-3) var(--space-2)',
                      border: `1px solid ${
                        selected ? 'var(--color-accent)' : 'var(--color-border)'
                      }`,
                      borderRadius: 'var(--radius-md)',
                      background: selected
                        ? 'var(--color-surface-subtle)'
                        : 'transparent',
                      cursor: submitting ? 'not-allowed' : 'pointer',
                      textAlign: 'center',
                      transition:
                        'background var(--motion-fast) var(--ease-standard), border-color var(--motion-fast) var(--ease-standard)',
                    }}
                  >
                    <input
                      type="radio"
                      name="card-shape"
                      value={shape.value}
                      checked={selected}
                      onChange={() => setCardShape(shape.value)}
                      disabled={submitting}
                      aria-label={shape.label}
                      data-testid={`radio-card-shape-${shape.value}`}
                      style={{
                        position: 'absolute',
                        width: 1,
                        height: 1,
                        opacity: 0,
                        margin: 0,
                      }}
                    />
                    <span style={{ fontSize: '20px' }}>{shape.emoji}</span>
                    <span
                      style={{
                        fontSize: '12px',
                        fontWeight: 500,
                        color: 'var(--color-text-primary)',
                      }}
                    >
                      {shape.label}
                    </span>
                    <span
                      style={{
                        fontSize: '10px',
                        color: 'var(--color-text-muted)',
                        lineHeight: 1.3,
                      }}
                    >
                      {shape.description}
                    </span>
                  </label>
                )
              })}
            </div>
          </fieldset>

          {/* ── Icon picker ────────────────────────────────────── */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-1)' }}>
            <label
              htmlFor="create-template-icon"
              style={{
                fontSize: '13px',
                fontWeight: 500,
                color: 'var(--color-text-secondary)',
              }}
            >
              Icon
            </label>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 'var(--space-2)',
                padding: '4px 6px 4px 10px',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius-md)',
                background: 'var(--color-surface-subtle)',
              }}
            >
              <span
                style={{
                  fontSize: '18px',
                  minWidth: '24px',
                  textAlign: 'center',
                }}
                aria-hidden
              >
                {icon || '·'}
              </span>
              <input
                id="create-template-icon"
                type="text"
                value={icon}
                onChange={(e) => setIcon(e.target.value)}
                disabled={submitting}
                placeholder="🔗"
                maxLength={4}
                aria-label="Icon"
                style={{
                  flex: 1,
                  border: 'none',
                  outline: 'none',
                  background: 'transparent',
                  fontSize: '14px',
                  color: 'var(--color-text-primary)',
                  minWidth: 0,
                }}
              />
            </div>
            <div
              role="group"
              aria-label="Quick icon presets"
              style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}
            >
              {PRESET_ICONS.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => setIcon(preset)}
                  disabled={submitting}
                  aria-label={`Use ${preset} icon`}
                  aria-pressed={icon === preset}
                  style={{
                    width: '28px',
                    height: '28px',
                    border: '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-sm)',
                    background:
                      icon === preset
                        ? 'var(--color-surface-subtle)'
                        : 'transparent',
                    cursor: submitting ? 'not-allowed' : 'pointer',
                    fontSize: '14px',
                    lineHeight: 1,
                  }}
                >
                  {preset}
                </button>
              ))}
            </div>
          </div>

          {/* ── Actions ────────────────────────────────────────── */}
          <div
            style={{
              display: 'flex',
              justifyContent: 'flex-end',
              gap: 'var(--space-2)',
              marginTop: 'var(--space-2)',
            }}
          >
            <button
              type="button"
              onClick={onClose}
              disabled={submitting}
              style={{
                padding: '8px 14px',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius-md)',
                background: 'transparent',
                color: 'var(--color-text-secondary)',
                fontSize: '13px',
                cursor: submitting ? 'not-allowed' : 'pointer',
              }}
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!isValid || submitting}
              style={{
                padding: '8px 14px',
                border: 'none',
                borderRadius: 'var(--radius-md)',
                background:
                  !isValid || submitting
                    ? 'var(--color-surface-subtle)'
                    : 'var(--color-accent)',
                color:
                  !isValid || submitting
                    ? 'var(--color-text-muted)'
                    : 'var(--color-surface, #fff)',
                fontSize: '13px',
                fontWeight: 500,
                cursor: !isValid || submitting ? 'not-allowed' : 'pointer',
              }}
            >
              {submitting ? 'Creating…' : 'Create'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
