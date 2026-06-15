/**
 * JournalAggregator — T7 of slash-command-functional-behavior.
 *
 * Renders 4 default query blocks at the bottom of the journal page when
 * the user opts in via the `journal.aggregate` setting.
 *
 * The 4 sections (per spec §12.1):
 *  1. NOW in progress  — `(and (task now) (task doing))`
 *  2. Scheduled today — `(and (scheduled today) (or (task todo) (task doing) (task waiting)))`
 *  3. Deadlines today — `(and (deadline today) (or (task todo) (task doing) (task waiting)))`
 *  4. Overdue         — `(overdue)` with task filter
 *
 * Each section executes its DSL via `api.executeQuery` and renders results
 * grouped by source page.
 */

import { useState, useEffect, useCallback } from 'react'
import { Clock, Calendar, AlertCircle, CheckCircle2 } from 'lucide-react'
import { api } from '@core/api-client'
import { searchApi } from '@core/api/search'
import type { UserSettings, Block } from '@shared/types/api'
import type { QueryAst, QueryResult } from '@shared/types/queryAst'

// ─── Section definitions ────────────────────────────────────────────────────

interface AggregationSection {
  /** Display heading text */
  heading: string
  /** h3 heading level */
  level: 3
  /** DSL expression AST */
  dsl: QueryAst
  /** Icon shown next to the heading */
  icon: React.ReactNode
}

const SECTIONS: AggregationSection[] = [
  {
    heading: 'NOW in progress',
    level: 3,
    dsl: { And: [{ Task: ['now'] }, { Task: ['doing'] }] },
    icon: <Clock size={14} />,
  },
  {
    heading: 'Scheduled today',
    level: 3,
    dsl: {
      And: [
        { Scheduled: { predicate: 'Today' } },
        { Or: [{ Task: ['todo'] }, { Task: ['doing'] }, { Task: ['waiting'] }] },
      ],
    },
    icon: <Calendar size={14} />,
  },
  {
    heading: 'Deadlines today',
    level: 3,
    dsl: {
      And: [
        { Deadline: { predicate: 'Today' } },
        { Or: [{ Task: ['todo'] }, { Task: ['doing'] }, { Task: ['waiting'] }] },
      ],
    },
    icon: <AlertCircle size={14} />,
  },
  {
    heading: 'Overdue',
    level: 3,
    dsl: { Or: [{ Task: ['todo'] }, { Task: ['doing'] }, { Task: ['waiting'] }] },
    icon: <CheckCircle2 size={14} />,
  },
]

// ─── Settings toggle ────────────────────────────────────────────────────────

interface SettingsToggleProps {
  value: boolean
  onChange: (value: boolean) => void
}

function SettingsToggle({ value, onChange }: SettingsToggleProps) {
  return (
    <label
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-2)',
        cursor: 'pointer',
        fontSize: '13px',
        color: 'var(--color-text-secondary)',
      }}
    >
      <span>Show daily aggregations</span>
      <button
        type="button"
        role="switch"
        aria-checked={value}
        aria-label="Daily aggregations"
        onClick={() => onChange(!value)}
        style={{
          width: '36px',
          height: '20px',
          borderRadius: '10px',
          background: value ? 'var(--color-accent)' : 'var(--color-border)',
          border: 'none',
          cursor: 'pointer',
          position: 'relative',
          transition: 'background 0.2s',
          flexShrink: 0,
        }}
      >
        <span
          style={{
            display: 'block',
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            background: 'white',
            position: 'absolute',
            top: '2px',
            left: value ? '18px' : '2px',
            transition: 'left 0.2s',
          }}
        />
      </button>
    </label>
  )
}

// ─── Section results ────────────────────────────────────────────────────────

interface SectionResultsProps {
  section: AggregationSection
  pageName: string
}

interface GroupedResults {
  [pageName: string]: Block[]
}

function SectionResults({ section, pageName }: SectionResultsProps) {
  const [result, setResult] = useState<QueryResult | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    setError(null)

    searchApi
      .executeQuery(section.dsl, 50)
      .then((res) => {
        if (!cancelled) {
          setResult(res)
          setLoading(false)
        }
      })
      .catch((err: Error) => {
        if (!cancelled) {
          setError(err.message ?? 'Query failed')
          setLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [section.dsl])

  if (loading) {
    return (
      <div style={{ padding: 'var(--space-2)', color: 'var(--color-text-muted)', fontSize: '12px' }}>
        Loading...
      </div>
    )
  }

  if (error) {
    return (
      <div
        role="alert"
        style={{
          padding: 'var(--space-2)',
          color: 'var(--color-danger)',
          fontSize: '12px',
        }}
      >
        ⚠️ Query error: {error}
      </div>
    )
  }

  if (!result || result.results.length === 0) {
    return (
      <div
        style={{
          padding: 'var(--space-2)',
          color: 'var(--color-text-muted)',
          fontSize: '12px',
          fontStyle: 'italic',
        }}
      >
        (none)
      </div>
    )
  }

  // Group results by pageName
  const grouped: GroupedResults = {}
  for (const row of result.results) {
    const page = (row.pageName as string) ?? 'Unknown'
    if (!grouped[page]) grouped[page] = []
    grouped[page].push(row as unknown as Block)
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
      {Object.entries(grouped).map(([groupPage, blocks]) => (
        <div key={groupPage} style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          {/* Group heading */}
          <div
            style={{
              fontSize: '11px',
              fontWeight: 600,
              color: 'var(--color-text-muted)',
              textTransform: 'uppercase',
              letterSpacing: '0.04em',
              paddingLeft: 'var(--space-2)',
              marginTop: 'var(--space-1)',
            }}
          >
            {groupPage}
          </div>
          {/* Block list */}
          {blocks.map((block) => (
            <div
              key={block.id}
              style={{
                padding: '4px var(--space-2)',
                fontSize: '13px',
                borderRadius: 'var(--radius-sm)',
              }}
            >
              <span
                style={{
                  opacity: block.marker === 'Done' || block.marker === 'Cancelled' ? 0.5 : 1,
                  textDecoration:
                    block.marker === 'Cancelled' ? 'line-through' : 'none',
                }}
              >
                {block.content || '(empty)'}
              </span>
            </div>
          ))}
        </div>
      ))}
    </div>
  )
}

// ─── Main component ─────────────────────────────────────────────────────────

interface JournalAggregatorProps {
  /** Journal page name (e.g. "2026-06-15") */
  pageName: string
}

export function JournalAggregator({ pageName }: JournalAggregatorProps) {
  const [settings, setSettings] = useState<UserSettings | null>(null)
  const [loading, setLoading] = useState(true)

  // Load settings on mount
  useEffect(() => {
    let cancelled = false
    api
      .getSettings()
      .then((s) => {
        if (!cancelled) setSettings(s)
      })
      .catch(() => {
        // Non-critical: default to off
        if (!cancelled) setSettings(null)
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [])

  const handleToggle = useCallback(
    async (value: boolean) => {
      // Optimistic update
      setSettings((prev) => (prev ? { ...prev, journalAggregate: value } : null))
      try {
        const updated = await api.updateSettings({ journalAggregate: value })
        setSettings(updated)
      } catch {
        // Revert on failure
        setSettings((prev) => (prev ? { ...prev, journalAggregate: !value } : null))
      }
    },
    [],
  )

  if (loading) return null

  // Default to false if setting is not present
  const enabled = settings?.journalAggregate ?? false

  return (
    <div
      data-testid="journal-aggregator"
      style={{
        marginTop: 'var(--space-8)',
        borderTop: '1px solid var(--color-border)',
        paddingTop: 'var(--space-4)',
      }}
    >
      {/* Settings toggle */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'flex-end',
          marginBottom: 'var(--space-4)',
        }}
      >
        <SettingsToggle value={enabled} onChange={handleToggle} />
      </div>

      {/* Only render sections when enabled */}
      {enabled && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-6)',
          }}
        >
          {SECTIONS.map((section) => (
            <section key={section.heading} aria-labelledby={`section-${section.heading}`}>
              <div
                id={`section-${section.heading}`}
                role="heading"
                aria-level={section.level}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-2)',
                  fontSize: '14px',
                  fontWeight: 600,
                  color: 'var(--color-text-primary)',
                  marginBottom: 'var(--space-2)',
                }}
              >
                {section.icon}
                <span>{section.heading}</span>
              </div>
              <SectionResults section={section} pageName={pageName} />
            </section>
          ))}
        </div>
      )}
    </div>
  )
}
