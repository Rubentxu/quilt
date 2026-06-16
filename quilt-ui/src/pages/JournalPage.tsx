import { useState, useEffect } from 'react'
import { useParams } from '@tanstack/react-router'
import { PageView } from '@features/outliner-tiptap/PageView'
import { ErrorBoundary } from '@shared/components/ErrorBoundary'
import { JournalAggregator } from '@features/journal/JournalAggregator'
import { MorningBriefing } from '@features/cognitive/MorningBriefing'
import { api, QuiltApiError } from '@core/api-client'
import { useWasm } from '@core/wasm-bridge/WasmProvider'
import { useTabs } from '@shared/contexts/TabsContext'
import type { Page, UserSettings } from '@shared/types/api'

/** Date format regex: YYYY-MM-DD */
const DATE_REGEX = /^\d{4}-\d{2}-\d{2}$/

function formatJournalDate(dateStr: string): string {
  try {
    const d = new Date(dateStr + 'T00:00:00')
    return d.toLocaleDateString('en-US', {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  } catch {
    return dateStr
  }
}

/** Format a journal date for the page title (long form) */
function formatJournalTitle(dateStr: string): string {
  try {
    const d = new Date(dateStr + 'T00:00:00')
    return d.toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    })
  } catch {
    return dateStr
  }
}

export function JournalPage() {
  const { date } = useParams({ from: '/journal/$date' })
  const { openTab } = useTabs()
  const [page, setPage] = useState<Page | null>(null)
  const [settings, setSettings] = useState<UserSettings | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Auto-open tab for this journal
  useEffect(() => {
    openTab({
      name: date,
      type: 'journal',
      title: formatJournalDate(date),
      params: { date },
    })
  }, [date, openTab])

  // Load or auto-create the journal page
  useEffect(() => {
    async function load() {
      setLoading(true)
      setError(null)

      // Validate date format upfront
      if (!DATE_REGEX.test(date)) {
        setError('Invalid date format. Expected YYYY-MM-DD.')
        setLoading(false)
        return
      }

      try {
        // getJournal auto-creates on the backend when the journal doesn't exist
        const [journalPage, userSettings] = await Promise.all([
          api.getJournal(date),
          api.getSettings(),
        ])
        setPage(journalPage)
        setSettings(userSettings)
      } catch (e) {
        if (e instanceof QuiltApiError && e.status === 404) {
          // Fallback: backend didn't auto-create, create explicitly
          try {
            const newPage = await api.createPage({
              name: date,
              title: formatJournalTitle(date),
              isJournal: true,
              journalDay: date.replace(/-/g, ''), // YYYYMMDD
            })
            setPage(newPage)
            // Load settings separately (non-critical)
            try {
              const s = await api.getSettings()
              setSettings(s)
            } catch {
              // settings are optional
            }
          } catch (createErr) {
            setError(createErr instanceof Error ? createErr.message : 'Failed to create journal')
          }
        } else {
          setError(e instanceof Error ? e.message : 'Failed to load journal')
        }
      } finally {
        setLoading(false)
      }
    }
    load()
  }, [date])

  if (loading) {
    return (
      <div style={{ padding: 'var(--space-8)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
        Loading journal...
      </div>
    )
  }

  if (error || !page) {
    return (
      <div style={{ padding: 'var(--space-8)', color: 'var(--color-danger)' }}>
        {error || 'Journal not found'}
      </div>
    )
  }

  return (
    <ErrorBoundary
      fallback={
        <div style={{ padding: 'var(--space-4)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
          <p style={{ fontSize: '14px', fontWeight: 600, color: 'var(--color-danger)' }}>Failed to load journal</p>
          <button onClick={() => window.location.reload()} style={{
            marginTop: 'var(--space-2)',
            padding: 'var(--space-2) var(--space-4)',
            background: 'var(--color-accent)',
            color: 'white',
            border: 'none',
            borderRadius: 'var(--radius-md)',
            cursor: 'pointer',
          }}>
            Retry
          </button>
        </div>
      }
    >
      <div style={{ padding: 'var(--space-3)' }}>
        <MorningBriefing />
      </div>
      <PageView
        pageName={page.name}
        isJournal
        journalFormat={settings?.journalFormat}
      />
      <JournalAggregator pageName={page.name} />
    </ErrorBoundary>
  )
}
