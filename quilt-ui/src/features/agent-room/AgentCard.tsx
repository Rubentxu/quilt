// ─── AgentCard ───────────────────────────────────────────────────────
//
// One card per agent in the AgentRoom list. Renders the
// status badge, summary/error preview, and (when the run is
// still active) a Cancel button. Status badge colours
// mirror `AgentRunRenderer` so the visual language is
// consistent across the app.

import { useState } from 'react'
import { Bot, Loader2, X } from 'lucide-react'
import type { AgentDto, AgentStatus } from '@shared/types/api'

interface AgentCardProps {
  agent: AgentDto
  onCancel?: (id: string) => void
}

const STATUS_STYLES: Record<AgentStatus, { bg: string; text: string; label: string }> = {
  Queued: { bg: 'var(--color-text-muted)', text: '#fff', label: 'QUEUED' },
  Running: { bg: 'var(--color-info)', text: '#fff', label: 'RUNNING' },
  Completed: { bg: 'var(--color-success)', text: '#fff', label: 'COMPLETED' },
  Failed: { bg: 'var(--color-danger)', text: '#fff', label: 'FAILED' },
  Cancelled: { bg: 'var(--color-text-disabled)', text: '#fff', label: 'CANCELLED' },
}

const ACTIVE_STATUSES: AgentStatus[] = ['Queued', 'Running']

function formatTime(iso: string | null | undefined): string {
  if (!iso) return '—'
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

export function AgentCard({ agent, onCancel }: AgentCardProps) {
  const [cancelling, setCancelling] = useState(false)
  const style = STATUS_STYLES[agent.status]
  const isActive = ACTIVE_STATUSES.includes(agent.status)

  async function handleCancel() {
    if (!onCancel) return
    setCancelling(true)
    try {
      await onCancel(agent.id)
    } finally {
      setCancelling(false)
    }
  }

  return (
    <div
      data-testid={`agent-card-${agent.id}`}
      data-agent-status={agent.status}
      style={{
        padding: 'var(--space-2)',
        borderBottom: '1px solid var(--color-border)',
        fontSize: '12px',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          marginBottom: '4px',
          flexWrap: 'wrap',
        }}
      >
        <span
          aria-hidden="true"
          style={{ display: 'inline-flex', alignItems: 'center', gap: '2px' }}
        >
          <Bot size={12} />
        </span>
        <span
          style={{
            fontWeight: 600,
            fontFamily: 'monospace',
            fontSize: '11px',
            color: 'var(--color-accent)',
          }}
        >
          {agent.agentType}
        </span>
        {agent.model && (
          <span
            data-testid={`agent-card-model-${agent.id}`}
            style={{ color: 'var(--color-text-muted)', fontSize: '10px' }}
          >
            {agent.model}
          </span>
        )}
        <span
          data-testid={`agent-card-status-${agent.id}`}
          aria-label={`Status: ${agent.status}`}
          style={{
            padding: '2px 6px',
            borderRadius: 'var(--radius-pill)',
            background: style.bg,
            color: style.text,
            fontSize: '10px',
            fontWeight: 600,
            letterSpacing: '0.02em',
          }}
        >
          {style.label}
        </span>
        {isActive && onCancel && (
          <button
            data-testid={`agent-card-cancel-${agent.id}`}
            aria-label={`Cancel ${agent.agentType} run`}
            disabled={cancelling}
            onClick={handleCancel}
            style={{
              marginLeft: 'auto',
              background: 'none',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: '2px 6px',
              fontSize: '10px',
              color: 'var(--color-text-muted)',
              cursor: cancelling ? 'default' : 'pointer',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '2px',
            }}
          >
            {cancelling ? (
              <Loader2 size={10} style={{ animation: 'spin 1s linear infinite' }} />
            ) : (
              <X size={10} />
            )}
            Cancel
          </button>
        )}
      </div>
      <div
        style={{
          color: 'var(--color-text-muted)',
          fontSize: '10px',
          marginBottom: '2px',
        }}
      >
        Started: {formatTime(agent.startedAt)}
        {agent.completedAt ? ` · Done: ${formatTime(agent.completedAt)}` : ''}
      </div>
      {agent.summary && (
        <div
          data-testid={`agent-card-summary-${agent.id}`}
          style={{ color: 'var(--color-text-secondary)', lineHeight: 1.4 }}
        >
          {agent.summary}
        </div>
      )}
      {agent.error && (
        <div
          data-testid={`agent-card-error-${agent.id}`}
          style={{ color: 'var(--color-danger, #c0392b)', lineHeight: 1.4 }}
        >
          {agent.error}
        </div>
      )}
      <div
        style={{
          color: 'var(--color-text-muted)',
          fontSize: '10px',
          marginTop: '2px',
        }}
      >
        blocks modified: {agent.blocksModified}
      </div>
    </div>
  )
}
