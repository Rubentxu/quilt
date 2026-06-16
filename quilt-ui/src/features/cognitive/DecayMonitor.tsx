// ─── DecayMonitor ────────────────────────────────────────────────────────
//
// CG-7: Decay Monitor end-to-end.
// Standalone panel that surfaces stale blocks from the knowledge
// graph. Renders four states (loading / error / empty / with-data)
// and groups alerts by severity (high → medium → low).
//
// Per ADR-0001, Quilt does NOT integrate AI/LLM providers.
// Decay detection is purely structural (a block's `updatedAt` vs now).

import { useCallback, useEffect, useState } from 'react'
import { AlertTriangle, Loader2 } from 'lucide-react'
import { api } from '@core/api-client'
import type {
  DecayMonitorDto,
  DecayAlert,
  SeverityCounts,
} from '@shared/types/api'

interface DecayMonitorProps {
  /** Called when the user clicks on an alert or presses Enter/Space. */
  onNavigate?: (blockId: string, pageName: string) => void
}

// ─── Helpers ────────────────────────────────────────────────────────────

function severityColor(severity: string): string {
  if (severity === 'high') return 'var(--color-danger, #c0392b)'
  if (severity === 'medium') return 'var(--color-warning, #e67e22)'
  return 'var(--color-text-muted)'
}

function severityLabel(severity: string): string {
  if (severity === 'high') return 'HIGH'
  if (severity === 'medium') return 'MEDIUM'
  return 'LOW'
}

function buildSeverityBuckets(alerts: DecayAlert[]): SeverityCounts {
  const counts: SeverityCounts = { low: 0, medium: 0, high: 0 }
  for (const alert of alerts) {
    if (alert.severity === 'high') counts.high += 1
    else if (alert.severity === 'medium') counts.medium += 1
    else counts.low += 1
  }
  return counts
}

// ─── Empty section ──────────────────────────────────────────────────────

function DecayEmpty() {
  return (
    <div
      data-testid="decay-monitor-empty"
      style={{
        padding: 'var(--space-2)',
        color: 'var(--color-text-muted)',
        fontSize: '12px',
        fontStyle: 'italic',
      }}
    >
      No decay alerts — everything looks healthy
    </div>
  )
}

// ─── Severity group ─────────────────────────────────────────────────────

interface SeverityGroupProps {
  severity: 'high' | 'medium' | 'low'
  alerts: DecayAlert[]
  onNavigate?: (blockId: string, pageName: string) => void
}

function SeverityGroup({ severity, alerts, onNavigate }: SeverityGroupProps) {
  return (
    <section
      role="region"
      aria-labelledby={`decay-monitor-${severity}-label`}
      data-testid={`decay-monitor-group-${severity}`}
      style={{ borderTop: '1px solid var(--color-border)' }}
    >
      <div
        id={`decay-monitor-${severity}-label`}
        data-testid={`decay-monitor-group-${severity}-header`}
        style={{
          fontSize: '11px',
          fontWeight: 600,
          color: severityColor(severity),
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          padding: 'var(--space-1) var(--space-2)',
          display: 'flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        <AlertTriangle size={12} color={severityColor(severity)} />
        {severityLabel(severity)}
        <span
          style={{
            marginLeft: 'auto',
            fontWeight: 400,
            textTransform: 'none',
            letterSpacing: 0,
            color: 'var(--color-text-muted)',
          }}
        >
          {alerts.length}
        </span>
      </div>
      <ul
        data-testid={`decay-monitor-list-${severity}`}
        style={{ listStyle: 'none', margin: 0, padding: 0 }}
        role="list"
      >
        {alerts.map((alert) => (
          <li
            key={alert.blockId}
            data-testid={`decay-monitor-item-${alert.blockId}`}
            role="listitem"
            style={{
              padding: 'var(--space-2)',
              borderBottom: '1px solid var(--color-border)',
              fontSize: '12px',
              cursor: onNavigate ? 'pointer' : 'default',
            }}
            onClick={() => onNavigate?.(alert.blockId, alert.pageName)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault()
                onNavigate?.(alert.blockId, alert.pageName)
              }
            }}
            tabIndex={onNavigate ? 0 : undefined}
            aria-label={`Decay alert: ${alert.contentPreview.slice(0, 60)} — ${alert.reason}`}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                marginBottom: '2px',
              }}
            >
              <span
                style={{
                  color: severityColor(severity),
                  fontWeight: 600,
                  fontSize: '11px',
                }}
              >
                {alert.daysSinceUpdate}d ago
              </span>
            </div>
            <div
              style={{ color: 'var(--color-text-secondary)', lineHeight: 1.4 }}
            >
              {alert.contentPreview.length > 100
                ? alert.contentPreview.slice(0, 100) + '…'
                : alert.contentPreview || '(empty block)'}
            </div>
            <div
              style={{
                color: 'var(--color-text-muted)',
                fontSize: '10px',
                marginTop: '2px',
              }}
            >
              {alert.pageName}
            </div>
          </li>
        ))}
      </ul>
    </section>
  )
}

// ─── Main component ─────────────────────────────────────────────────────

export function DecayMonitor({ onNavigate }: DecayMonitorProps) {
  const [data, setData] = useState<DecayMonitorDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    setError(null)
    try {
      const result = await api.getDecayAlerts()
      setData(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load decay alerts')
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [])

  useEffect(() => {
    void load(false)
  }, [load])

  // Group alerts by severity for the rendering.
  const grouped = data
    ? {
        high: data.alerts.filter((a) => a.severity === 'high'),
        medium: data.alerts.filter((a) => a.severity === 'medium'),
        low: data.alerts.filter((a) => a.severity === 'low'),
      }
    : null

  // Use the server-provided counts when available; fall back to
  // recomputing in case the server returned a partial DTO.
  const counts =
    data?.countsBySeverity ??
    (data ? buildSeverityBuckets(data.alerts) : { low: 0, medium: 0, high: 0 })

  return (
    <div
      data-testid="decay-monitor"
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
      }}
      role="region"
      aria-label="Decay Monitor"
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
          <AlertTriangle size={16} color="var(--color-warning, #e67e22)" />
          <span style={{ fontWeight: 600, fontSize: '14px' }}>Decay Monitor</span>
          {data && (
            <span
              data-testid="decay-monitor-counts"
              style={{ color: 'var(--color-text-muted)', fontSize: '11px' }}
            >
              {counts.high} high · {counts.medium} medium · {counts.low} low
            </span>
          )}
        </div>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh decay monitor"
          data-testid="decay-monitor-refresh"
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
          data-testid="decay-monitor-loading"
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
            Loading decay…
          </div>
        </div>
      )}

      {/* Error state */}
      {error && !loading && (
        <div
          data-testid="decay-monitor-error"
          style={{
            padding: 'var(--space-3)',
            color: 'var(--color-danger, #c0392b)',
            fontSize: '12px',
          }}
        >
          {error}
        </div>
      )}

      {/* Content */}
      {!loading && !error && data && (
        <div>
          {data.alerts.length === 0 ? (
            <DecayEmpty />
          ) : (
            grouped && (
              <>
                {grouped.high.length > 0 && (
                  <SeverityGroup
                    severity="high"
                    alerts={grouped.high}
                    onNavigate={onNavigate}
                  />
                )}
                {grouped.medium.length > 0 && (
                  <SeverityGroup
                    severity="medium"
                    alerts={grouped.medium}
                    onNavigate={onNavigate}
                  />
                )}
                {grouped.low.length > 0 && (
                  <SeverityGroup
                    severity="low"
                    alerts={grouped.low}
                    onNavigate={onNavigate}
                  />
                )}
              </>
            )
          )}

          {/* Generated at footer */}
          <div
            data-testid="decay-monitor-footer"
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
