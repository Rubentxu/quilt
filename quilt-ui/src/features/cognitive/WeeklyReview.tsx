// ─── WeeklyReview ────────────────────────────────────────────────────────
//
// CG-7: Weekly Review end-to-end.
// Guided-workflow panel that walks the user through a 4-step
// review of the last 7 days:
//
//   1. Numbers — blocks created, updated, tasks completed, journal days
//   2. Decay   — current decay count, trend, delta
//   3. Suggestions — heuristic "what to focus on next week"
//   4. Next week — closing prompt
//
// Per ADR-0001, no AI/LLM integration. The suggestions heuristic
// lives in the backend (`quilt-analysis::weekly_review::service`);
// the UI just renders.

import { useCallback, useEffect, useState } from 'react'
import {
  Calendar,
  ChevronLeft,
  ChevronRight,
  Check,
  TrendingUp,
  TrendingDown,
  Minus,
  Loader2,
} from 'lucide-react'
import { api } from '@core/api-client'
import type { WeeklyReviewDto, DecayTrend } from '@shared/types/api'

interface WeeklyReviewProps {
  /** Optional navigation callback (unused in V1, kept for parity with other panels). */
  onNavigate?: (blockId: string, pageName: string) => void
}

const TOTAL_STEPS = 4

// ─── Helpers ────────────────────────────────────────────────────────────

function trendColor(trend: DecayTrend): string {
  if (trend === 'worsening') return 'var(--color-danger, #c0392b)'
  if (trend === 'improving') return 'var(--color-accent, #4f46e5)'
  return 'var(--color-text-muted)'
}

function trendIcon(trend: DecayTrend) {
  if (trend === 'worsening') return TrendingDown
  if (trend === 'improving') return TrendingUp
  return Minus
}

function formatDateRange(weekStart: string, weekEnd: string): string {
  try {
    const s = new Date(weekStart)
    const e = new Date(weekEnd)
    const opts: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric' }
    return `${s.toLocaleDateString(undefined, opts)} – ${e.toLocaleDateString(
      undefined,
      opts,
    )}, ${e.getFullYear()}`
  } catch {
    return `${weekStart} – ${weekEnd}`
  }
}

// ─── Main component ─────────────────────────────────────────────────────

export function WeeklyReview(_props: WeeklyReviewProps) {
  const [data, setData] = useState<WeeklyReviewDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [step, setStep] = useState<number>(1)

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    setError(null)
    try {
      const result = await api.getWeeklyReview()
      setData(result)
      setStep(1)
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to load weekly review',
      )
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [])

  useEffect(() => {
    void load(false)
  }, [load])

  const isEmpty =
    !!data &&
    data.blocksCreated === 0 &&
    data.blocksUpdated === 0 &&
    data.tasksCompleted === 0 &&
    data.journalDays === 0

  const advance = useCallback(() => {
    setStep((s) => Math.min(s + 1, TOTAL_STEPS))
  }, [])

  const retreat = useCallback(() => {
    setStep((s) => Math.max(s - 1, 1))
  }, [])

  const finish = useCallback(() => {
    setStep(1)
  }, [])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault()
        if (step < TOTAL_STEPS) advance()
        else finish()
      } else if (e.key === 'ArrowRight') {
        e.preventDefault()
        advance()
      } else if (e.key === 'ArrowLeft') {
        e.preventDefault()
        retreat()
      } else if (e.key === 'Escape') {
        e.preventDefault()
        finish()
      }
    },
    [step, advance, retreat, finish],
  )

  return (
    <div
      data-testid="weekly-review"
      role="region"
      aria-label="Weekly Review"
      tabIndex={0}
      onKeyDown={handleKeyDown}
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
        outline: 'none',
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: 'var(--space-3)',
          borderBottom: '1px solid var(--color-border)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Calendar size={16} color="var(--color-accent)" />
          <span style={{ fontWeight: 600, fontSize: '14px' }}>Weekly Review</span>
          {data && (
            <span
              data-testid="weekly-review-range"
              style={{ color: 'var(--color-text-muted)', fontSize: '11px' }}
            >
              {formatDateRange(data.weekStart, data.weekEnd)}
            </span>
          )}
        </div>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh weekly review"
          data-testid="weekly-review-refresh"
          style={{
            background: 'none',
            border: 'none',
            cursor: refreshing ? 'default' : 'pointer',
            color: 'var(--color-text-muted)',
            display: 'inline-flex',
            alignItems: 'center',
            padding: '4px',
            borderRadius: 'var(--radius-sm)',
          }}
        >
          <Loader2
            size={14}
            style={{
              animation: refreshing ? 'spin 1s linear infinite' : 'none',
            }}
          />
        </button>
      </div>

      {/* Loading state */}
      {loading && (
        <div
          data-testid="weekly-review-loading"
          style={{ padding: 'var(--space-4)', textAlign: 'center' }}
        >
          <Loader2
            size={16}
            color="var(--color-text-muted)"
            style={{ animation: 'spin 1s linear infinite' }}
          />
          <div
            style={{
              color: 'var(--color-text-muted)',
              fontSize: '12px',
              marginTop: 'var(--space-2)',
            }}
          >
            Loading review…
          </div>
        </div>
      )}

      {/* Error state */}
      {error && !loading && (
        <div
          data-testid="weekly-review-error"
          style={{
            padding: 'var(--space-3)',
            color: 'var(--color-danger, #c0392b)',
            fontSize: '12px',
          }}
        >
          {error}
        </div>
      )}

      {/* Empty state */}
      {!loading && !error && isEmpty && (
        <div
          data-testid="weekly-review-empty"
          style={{
            padding: 'var(--space-4)',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            fontStyle: 'italic',
            textAlign: 'center',
          }}
        >
          Start journaling to see your weekly review
        </div>
      )}

      {/* With data — guided steps */}
      {!loading && !error && data && !isEmpty && (
        <div>
          {/* Step 1: Numbers */}
          {step === 1 && (
            <section
              data-testid="weekly-review-step-1"
              role="region"
              aria-label="Step 1 of 4: Numbers"
              style={{ padding: 'var(--space-3)' }}
            >
              <h3
                style={{
                  fontSize: '12px',
                  fontWeight: 600,
                  color: 'var(--color-text-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  margin: '0 0 var(--space-3) 0',
                }}
              >
                Numbers
              </h3>
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: '1fr 1fr',
                  gap: 'var(--space-2)',
                }}
              >
                {(
                  [
                    ['blocksCreated', 'Blocks created'],
                    ['blocksUpdated', 'Blocks updated'],
                    ['tasksCompleted', 'Tasks completed'],
                    ['journalDays', 'Journal days'],
                  ] as const
                ).map(([key, label]) => (
                  <div
                    key={key}
                    data-testid={`weekly-review-counter-${key}`}
                    style={{
                      padding: 'var(--space-2)',
                      background: 'var(--color-bg-alt, #f9fafb)',
                      borderRadius: 'var(--radius-md)',
                      border: '1px solid var(--color-border)',
                    }}
                  >
                    <div
                      style={{
                        fontSize: '10px',
                        color: 'var(--color-text-muted)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.05em',
                      }}
                    >
                      {label}
                    </div>
                    <div
                      style={{
                        fontSize: '20px',
                        fontWeight: 600,
                        color: 'var(--color-text-primary)',
                        marginTop: '2px',
                      }}
                    >
                      {key === 'blocksCreated'
                        ? data.blocksCreated
                        : key === 'blocksUpdated'
                          ? data.blocksUpdated
                          : key === 'tasksCompleted'
                            ? data.tasksCompleted
                            : data.journalDays}
                    </div>
                  </div>
                ))}
              </div>
            </section>
          )}

          {/* Step 2: Decay */}
          {step === 2 && (
            <section
              data-testid="weekly-review-step-2"
              role="region"
              aria-label="Step 2 of 4: Decay"
              style={{ padding: 'var(--space-3)' }}
            >
              <h3
                style={{
                  fontSize: '12px',
                  fontWeight: 600,
                  color: 'var(--color-text-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  margin: '0 0 var(--space-3) 0',
                }}
              >
                Decay
              </h3>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-2)',
                  padding: 'var(--space-3)',
                  background: 'var(--color-bg-alt, #f9fafb)',
                  borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--color-border)',
                }}
              >
                {(() => {
                  const Icon = trendIcon(data.decayTrend)
                  return (
                    <Icon size={20} color={trendColor(data.decayTrend)} />
                  )
                })()}
                <div>
                  <div
                    data-testid="weekly-review-trend"
                    style={{
                      fontSize: '14px',
                      fontWeight: 600,
                      color: trendColor(data.decayTrend),
                    }}
                  >
                    {data.decayTrend}
                  </div>
                  <div
                    data-testid="weekly-review-delta"
                    style={{
                      fontSize: '11px',
                      color: 'var(--color-text-muted)',
                      marginTop: '2px',
                    }}
                  >
                    {data.decayDelta > 0
                      ? `${data.decayDelta} fewer decay alerts than last week`
                      : data.decayDelta < 0
                        ? `${-data.decayDelta} more decay alerts than last week`
                        : 'No change from last week'}
                  </div>
                </div>
              </div>
            </section>
          )}

          {/* Step 3: Suggestions */}
          {step === 3 && (
            <section
              data-testid="weekly-review-step-3"
              role="region"
              aria-label="Step 3 of 4: Suggestions"
              style={{ padding: 'var(--space-3)' }}
            >
              <h3
                style={{
                  fontSize: '12px',
                  fontWeight: 600,
                  color: 'var(--color-text-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  margin: '0 0 var(--space-3) 0',
                }}
              >
                Suggestions
              </h3>
              {data.suggestions.length === 0 ? (
                <div
                  style={{
                    color: 'var(--color-text-muted)',
                    fontSize: '12px',
                    fontStyle: 'italic',
                  }}
                >
                  Everything looks healthy
                </div>
              ) : (
                <ul
                  style={{
                    listStyle: 'none',
                    margin: 0,
                    padding: 0,
                  }}
                  role="list"
                >
                  {data.suggestions.map((s, i) => (
                    <li
                      key={i}
                      data-testid={`weekly-review-suggestion-${i}`}
                      style={{
                        padding: 'var(--space-2) 0',
                        borderBottom:
                          i < data.suggestions.length - 1
                            ? '1px solid var(--color-border)'
                            : 'none',
                        fontSize: '12px',
                        color: 'var(--color-text-secondary)',
                        lineHeight: 1.4,
                      }}
                    >
                      {s}
                    </li>
                  ))}
                </ul>
              )}
            </section>
          )}

          {/* Step 4: Next week */}
          {step === 4 && (
            <section
              data-testid="weekly-review-step-4"
              role="region"
              aria-label="Step 4 of 4: Next week"
              style={{ padding: 'var(--space-3)' }}
            >
              <h3
                style={{
                  fontSize: '12px',
                  fontWeight: 600,
                  color: 'var(--color-text-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  margin: '0 0 var(--space-3) 0',
                }}
              >
                Next week
              </h3>
              <div
                data-testid="weekly-review-prompt"
                style={{
                  fontSize: '13px',
                  color: 'var(--color-text-primary)',
                  lineHeight: 1.5,
                  padding: 'var(--space-3)',
                  background: 'var(--color-bg-alt, #f9fafb)',
                  borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--color-border)',
                }}
              >
                {data.suggestions.length > 0
                  ? `Focus on: ${data.suggestions[0]}`
                  : 'Keep going next week — the graph is healthy.'}
              </div>
            </section>
          )}

          {/* Navigation */}
          <div
            style={{
              padding: 'var(--space-2) var(--space-3)',
              borderTop: '1px solid var(--color-border)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: 'var(--space-2)',
            }}
          >
            {step > 1 ? (
              <button
                onClick={retreat}
                data-testid="weekly-review-back"
                aria-label={`Back to step ${step - 1}`}
                style={navButtonStyle('secondary')}
              >
                <ChevronLeft size={12} /> Back
              </button>
            ) : (
              <span
                data-testid="weekly-review-back-placeholder"
                style={{ width: 1 }}
              />
            )}

            <span
              data-testid="weekly-review-step-indicator"
              style={{
                fontSize: '11px',
                color: 'var(--color-text-muted)',
              }}
            >
              Step {step} of {TOTAL_STEPS}
            </span>

            {step < TOTAL_STEPS ? (
              <button
                onClick={advance}
                data-testid="weekly-review-next"
                aria-label={`Next to step ${step + 1}`}
                style={navButtonStyle('primary')}
              >
                Next <ChevronRight size={12} />
              </button>
            ) : (
              <button
                onClick={finish}
                data-testid="weekly-review-done"
                aria-label="Finish weekly review"
                style={navButtonStyle('primary')}
              >
                Done <Check size={12} />
              </button>
            )}
          </div>

          {/* Generated at footer */}
          <div
            data-testid="weekly-review-footer"
            style={{
              padding: 'var(--space-2)',
              borderTop: '1px solid var(--color-border)',
              color: 'var(--color-text-muted)',
              fontSize: '10px',
              textAlign: 'center',
            }}
          >
            Generated {new Date(data.generatedAt).toLocaleString()}
          </div>
        </div>
      )}
    </div>
  )
}

function navButtonStyle(
  kind: 'primary' | 'secondary',
): React.CSSProperties {
  return {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '4px',
    padding: '4px 8px',
    borderRadius: 'var(--radius-sm)',
    border: '1px solid var(--color-border)',
    background: kind === 'primary' ? 'var(--color-accent)' : 'transparent',
    color: kind === 'primary' ? '#fff' : 'var(--color-text-primary)',
    cursor: 'pointer',
    fontSize: '12px',
    fontWeight: 500,
  }
}
