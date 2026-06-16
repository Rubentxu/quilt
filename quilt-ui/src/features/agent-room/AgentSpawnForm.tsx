// ─── AgentSpawnForm ──────────────────────────────────────────────
//
// Form used by `AgentRoom` to spawn a new agent run. V1 ships
// ONE agent type (`decay-annotator`); the `<select>` is a real
// `<select>` so V2 types do not require a component change.

import { useState, type FormEvent } from 'react'
import type { SpawnAgentRequest } from '@shared/types/api'

interface AgentSpawnFormProps {
  /** Available agent types. In V1 this is `['decay-annotator']`. */
  availableTypes: string[]
  /** Called with the request when the user submits. */
  onSubmit: (req: SpawnAgentRequest) => Promise<void> | void
  /** Disable the submit button while a spawn is in flight. */
  submitting?: boolean
  /** Optional error from the last submit attempt. */
  error?: string | null
}

export function AgentSpawnForm({
  availableTypes,
  onSubmit,
  submitting = false,
  error = null,
}: AgentSpawnFormProps) {
  const [agentType, setAgentType] = useState(availableTypes[0] ?? '')
  const [contextPage, setContextPage] = useState('')

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    if (submitting) return
    const req: SpawnAgentRequest = { agentType }
    if (contextPage.trim().length > 0) {
      req.contextPage = contextPage.trim()
    }
    await onSubmit(req)
    setContextPage('')
  }

  return (
    <form
      data-testid="agent-room-spawn-form"
      onSubmit={handleSubmit}
      style={{
        padding: 'var(--space-2)',
        borderBottom: '1px solid var(--color-border)',
        display: 'flex',
        flexDirection: 'column',
        gap: '6px',
      }}
    >
      <label
        style={{
          fontSize: '10px',
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
        }}
      >
        Type
        <select
          data-testid="agent-room-spawn-type"
          aria-label="Agent type"
          value={agentType}
          onChange={(e) => setAgentType(e.target.value)}
          disabled={submitting}
          style={{
            marginTop: '2px',
            width: '100%',
            padding: '4px',
            fontSize: '12px',
            background: 'var(--color-surface)',
            color: 'var(--color-text)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-sm)',
          }}
        >
          {availableTypes.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </label>
      <label
        style={{
          fontSize: '10px',
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
        }}
      >
        Page (optional)
        <input
          data-testid="agent-room-spawn-context"
          aria-label="Context page (optional)"
          type="text"
          value={contextPage}
          onChange={(e) => setContextPage(e.target.value)}
          placeholder="e.g. journals/2024-01-15"
          disabled={submitting}
          style={{
            marginTop: '2px',
            width: '100%',
            padding: '4px',
            fontSize: '12px',
            background: 'var(--color-surface)',
            color: 'var(--color-text)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-sm)',
          }}
        />
      </label>
      <button
        type="submit"
        data-testid="agent-room-spawn-submit"
        disabled={submitting || agentType.length === 0}
        style={{
          padding: '6px',
          fontSize: '12px',
          fontWeight: 600,
          background: submitting
            ? 'var(--color-text-muted)'
            : 'var(--color-accent)',
          color: '#fff',
          border: 'none',
          borderRadius: 'var(--radius-sm)',
          cursor: submitting ? 'default' : 'pointer',
        }}
      >
        {submitting ? 'Spawning…' : 'Spawn'}
      </button>
      {error && (
        <div
          data-testid="agent-room-spawn-error"
          style={{
            color: 'var(--color-danger, #c0392b)',
            fontSize: '11px',
          }}
        >
          {error}
        </div>
      )}
    </form>
  )
}
