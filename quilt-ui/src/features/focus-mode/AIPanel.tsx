// ─── AIPanel — slide-in AI panel for focus mode ───────────────────
//
// Shows contextual insights for the current page:
//   - Decay alerts (blocks that haven't been updated in a while)
//   - Serendipity highlights (unexpected connections)
//   - Related blocks (from the same page or linked pages)
//
// Plus an "Ask AI" input that spawns a new agent run via CG-5's
// POST /api/v1/agents endpoint. Shows the agent's status and
// summary once it completes.

import { useCallback, useEffect, useState } from 'react'
import { X, Sparkles, AlertTriangle, Zap, Loader } from 'lucide-react'
import { api } from '@core/api-client'
import { useFocusMode } from './FocusModeContext'
import type { AgentDto, DecayMonitorDto, SerendipityResponseDto } from '@shared/types/api'

interface AIPanelProps {
  /** Current page name — used to filter related insights. */
  pageName?: string
}

interface InsightCardProps {
  title: string
  icon: React.ReactNode
  children: React.ReactNode
  variant?: 'default' | 'warning' | 'highlight'
}

function InsightCard({ title, icon, children, variant = 'default' }: InsightCardProps) {
  const borderColor = variant === 'warning'
    ? 'var(--color-danger)'
    : variant === 'highlight'
      ? 'var(--color-accent)'
      : 'var(--color-border)'

  return (
    <div
      data-testid="insight-card"
      style={{
        borderLeft: `3px solid ${borderColor}`,
        padding: 'var(--space-3) var(--space-4)',
        marginBottom: 'var(--space-3)',
        background: 'var(--color-surface-raised)',
        borderRadius: 'var(--radius-sm)',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          marginBottom: 'var(--space-2)',
          color: 'var(--color-text-secondary)',
          fontSize: '12px',
          fontWeight: 600,
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
        }}
      >
        {icon}
        <span>{title}</span>
      </div>
      {children}
    </div>
  )
}

/** Fetch and display decay alerts. */
function DecayInsights({ pageName }: { pageName?: string }) {
  const [decay, setDecay] = useState<DecayMonitorDto | null>(null)

  useEffect(() => {
    api.getDecayAlerts()
      .then(setDecay)
      .catch(() => setDecay(null))
  }, [])

  if (!decay) {
    return (
      <InsightCard title="Decay Alerts" icon={<AlertTriangle size={12} />} variant="warning">
        <p style={{ color: 'var(--color-text-muted)', fontSize: '13px', margin: 0 }}>
          Loading...
        </p>
      </InsightCard>
    )
  }

  if (decay.alerts.length === 0) {
    return (
      <InsightCard title="Decay Alerts" icon={<AlertTriangle size={12} />} variant="default">
        <p style={{ color: 'var(--color-text-muted)', fontSize: '13px', margin: 0 }}>
          No decay alerts. Pages are fresh.
        </p>
      </InsightCard>
    )
  }

  return (
    <InsightCard title="Decay Alerts" icon={<AlertTriangle size={12} />} variant="warning">
      <ul style={{ margin: 0, paddingLeft: 'var(--space-4)', fontSize: '13px' }}>
        {decay.alerts.slice(0, 3).map((alert) => (
          <li key={alert.blockId} style={{ marginBottom: 'var(--space-1)' }}>
            <span style={{ color: 'var(--color-text-secondary)' }}>
              {alert.contentPreview.slice(0, 60)}
              {alert.contentPreview.length > 60 ? '…' : ''}
            </span>
            <span style={{ color: 'var(--color-text-muted)', fontSize: '11px', marginLeft: 'var(--space-2)' }}>
              {alert.daysSinceUpdate}d old
            </span>
          </li>
        ))}
      </ul>
    </InsightCard>
  )
}

/** Fetch and display serendipity highlights. */
function SerendipityInsights() {
  const [serendipity, setSerendipity] = useState<SerendipityResponseDto | null>(null)

  useEffect(() => {
    api.getSerendipity()
      .then(setSerendipity)
      .catch(() => setSerendipity(null))
  }, [])

  if (!serendipity) {
    return (
      <InsightCard title="Serendipity" icon={<Zap size={12} />} variant="highlight">
        <p style={{ color: 'var(--color-text-muted)', fontSize: '13px', margin: 0 }}>
          Loading...
        </p>
      </InsightCard>
    )
  }

  if (serendipity.highlights.length === 0) {
    return (
      <InsightCard title="Serendipity" icon={<Zap size={12} />} variant="highlight">
        <p style={{ color: 'var(--color-text-muted)', fontSize: '13px', margin: 0 }}>
          No serendipity highlights yet.
        </p>
      </InsightCard>
    )
  }

  return (
    <InsightCard title="Serendipity" icon={<Zap size={12} />} variant="highlight">
      <ul style={{ margin: 0, paddingLeft: 'var(--space-4)', fontSize: '13px' }}>
        {serendipity.highlights.slice(0, 3).map((h, i) => (
          <li key={i} style={{ marginBottom: 'var(--space-1)' }}>
            <span style={{ color: 'var(--color-text-secondary)' }}>
              {h.explanation.slice(0, 80)}
              {h.explanation.length > 80 ? '…' : ''}
            </span>
          </li>
        ))}
      </ul>
    </InsightCard>
  )
}

/** "Ask AI" input + agent status display. */
function AskAI({ pageName }: { pageName?: string }) {
  const [question, setQuestion] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [latestAgent, setLatestAgent] = useState<AgentDto | null>(null)
  const [error, setError] = useState<string | null>(null)

  const handleSubmit = useCallback(async (e: React.FormEvent) => {
    e.preventDefault()
    if (!question.trim()) return

    setSubmitting(true)
    setError(null)

    try {
      const agent = await api.spawnAgent({
        agentType: 'decay-annotator', // CG-5 default agent type
        contextPage: pageName ?? undefined,
      })
      setLatestAgent(agent)
      setQuestion('')

      // Poll for completion
      const pollInterval = setInterval(async () => {
        try {
          const updated = await api.getAgent(agent.id)
          setLatestAgent(updated)
          if (updated.status !== 'Running' && updated.status !== 'Queued') {
            clearInterval(pollInterval)
          }
        } catch {
          clearInterval(pollInterval)
        }
      }, 2000)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to spawn agent')
    } finally {
      setSubmitting(false)
    }
  }, [question, pageName])

  const statusColor = latestAgent?.status === 'Completed'
    ? 'var(--color-success)'
    : latestAgent?.status === 'Failed'
      ? 'var(--color-danger)'
      : 'var(--color-text-muted)'

  return (
    <div data-testid="ask-ai">
      <form onSubmit={handleSubmit} style={{ display: 'flex', gap: 'var(--space-2)', marginBottom: 'var(--space-3)' }}>
        <input
          type="text"
          value={question}
          onChange={e => setQuestion(e.target.value)}
          placeholder="Ask AI about this page…"
          disabled={submitting}
          data-testid="ask-ai-input"
          style={{
            flex: 1,
            padding: 'var(--space-2) var(--space-3)',
            borderRadius: 'var(--radius-md)',
            border: '1px solid var(--color-border)',
            background: 'var(--color-surface)',
            color: 'var(--color-text)',
            fontSize: '14px',
            outline: 'none',
          }}
        />
        <button
          type="submit"
          disabled={submitting || !question.trim()}
          data-testid="ask-ai-submit"
          style={{
            padding: 'var(--space-2) var(--space-3)',
            borderRadius: 'var(--radius-md)',
            border: 'none',
            background: 'var(--color-primary)',
            color: 'white',
            cursor: submitting ? 'not-allowed' : 'pointer',
            opacity: submitting ? 0.6 : 1,
            fontSize: '14px',
          }}
        >
          {submitting ? <Loader size={14} className="spin" /> : <Sparkles size={14} />}
        </button>
      </form>

      {error && (
        <p data-testid="ask-ai-error" style={{ color: 'var(--color-danger)', fontSize: '12px', margin: 0 }}>
          {error}
        </p>
      )}

      {latestAgent && (
        <div
          data-testid="agent-status"
          style={{
            padding: 'var(--space-2) var(--space-3)',
            background: 'var(--color-surface-raised)',
            borderRadius: 'var(--radius-sm)',
            border: '1px solid var(--color-border)',
            fontSize: '13px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', marginBottom: 'var(--space-1)' }}>
            <span style={{ color: statusColor, fontWeight: 600 }}>
              {latestAgent.status}
            </span>
            {latestAgent.agentType && (
              <span style={{ color: 'var(--color-text-muted)' }}>
                · {latestAgent.agentType}
              </span>
            )}
          </div>
          {latestAgent.summary && (
            <p style={{ margin: 0, color: 'var(--color-text-secondary)', fontSize: '12px' }}>
              {latestAgent.summary}
            </p>
          )}
          {latestAgent.error && (
            <p style={{ margin: 0, color: 'var(--color-danger)', fontSize: '12px' }}>
              {latestAgent.error}
            </p>
          )}
        </div>
      )}
    </div>
  )
}

/**
 * AI panel — slides in from the right in focus mode.
 * Shows contextual insights and an "Ask AI" chat interface.
 */
export function AIPanel({ pageName }: AIPanelProps) {
  const { setAIPanelOpen } = useFocusMode()

  return (
    <aside
      data-testid="ai-panel"
      style={{
        width: '320px',
        flexShrink: 0,
        borderLeft: '1px solid var(--color-border)',
        background: 'var(--color-surface)',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        animation: 'slideInRight 200ms ease-out',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: 'var(--space-3) var(--space-4)',
          borderBottom: '1px solid var(--color-border)',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <Sparkles size={16} style={{ color: 'var(--color-primary)' }} />
          <span style={{ fontWeight: 600, fontSize: '14px' }}>AI Insights</span>
        </div>
        <button
          onClick={() => setAIPanelOpen(false)}
          data-testid="ai-panel-close"
          aria-label="Close AI panel"
          style={{
            background: 'transparent',
            border: 'none',
            cursor: 'pointer',
            padding: 'var(--space-1)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--color-text-muted)',
            display: 'flex',
            alignItems: 'center',
          }}
        >
          <X size={16} />
        </button>
      </div>

      {/* Scrollable content */}
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: 'var(--space-4)',
        }}
      >
        {/* Ask AI */}
        <div style={{ marginBottom: 'var(--space-4)' }}>
          <AskAI pageName={pageName} />
        </div>

        {/* Divider */}
        <div style={{ height: '1px', background: 'var(--color-border)', margin: 'var(--space-4) 0' }} />

        {/* Insights */}
        <DecayInsights pageName={pageName} />
        <SerendipityInsights />
      </div>

      <style>{`
        @keyframes slideInRight {
          from { transform: translateX(100%); }
          to { transform: translateX(0); }
        }
        .spin {
          animation: spin 1s linear infinite;
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </aside>
  )
}
