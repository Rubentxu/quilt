import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { ArrowLeft, Save } from 'lucide-react'
import { api } from '@core/api-client'
import { ErrorBoundary } from '@shared/components/ErrorBoundary'
import { useTabs } from '@shared/contexts/TabsContext'
import type { UserSettings, DateFormatOption } from '@shared/types/api'
import toast from 'react-hot-toast'

// ─── Card wrapper ──────────────────────────────────────────────

function SettingsCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        padding: 'var(--space-6)',
        marginBottom: 'var(--space-6)',
      }}
    >
      <h2
        style={{
          fontSize: '16px',
          fontWeight: 600,
          color: 'var(--color-text-primary)',
          marginBottom: 'var(--space-4)',
          paddingBottom: 'var(--space-3)',
          borderBottom: '1px solid var(--color-border)',
        }}
      >
        {title}
      </h2>
      {children}
    </section>
  )
}

// ─── Form field ────────────────────────────────────────────────

function FormField({
  label,
  hint,
  children,
}: {
  label: string
  hint?: string
  children: React.ReactNode
}) {
  return (
    <div style={{ marginBottom: 'var(--space-4)' }}>
      <label
        style={{
          display: 'block',
          fontSize: '13px',
          fontWeight: 600,
          color: 'var(--color-text-secondary)',
          marginBottom: 'var(--space-1)',
        }}
      >
        {label}
      </label>
      {children}
      {hint && (
        <p
          style={{
            fontSize: '11px',
            color: 'var(--color-text-muted)',
            marginTop: 'var(--space-1)',
          }}
        >
          {hint}
        </p>
      )}
    </div>
  )
}

// ─── Select wrapper ────────────────────────────────────────────

function Select({
  value,
  onChange,
  options,
  placeholder,
}: {
  value: string
  onChange: (val: string) => void
  options: { value: string; label: string }[]
  placeholder?: string
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      style={{
        width: '100%',
        maxWidth: '400px',
        padding: '8px var(--space-3)',
        fontSize: '13px',
        color: 'var(--color-text-primary)',
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        outline: 'none',
        fontFamily: 'inherit',
        cursor: 'pointer',
        transition: 'border-color var(--motion-fast) var(--ease-standard)',
      }}
      className="settings-select"
    >
      {placeholder && (
        <option value="" disabled>
          {placeholder}
        </option>
      )}
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  )
}

// ─── Timezone options ──────────────────────────────────────────

const TIMEZONE_OPTIONS = Intl.supportedValuesOf
  ? Intl.supportedValuesOf('timeZone').map((tz: string) => ({ value: tz, label: tz }))
  : [
      { value: 'UTC', label: 'UTC' },
      { value: 'America/New_York', label: 'America/New_York' },
      { value: 'America/Chicago', label: 'America/Chicago' },
      { value: 'America/Denver', label: 'America/Denver' },
      { value: 'America/Los_Angeles', label: 'America/Los_Angeles' },
      { value: 'Europe/London', label: 'Europe/London' },
      { value: 'Europe/Madrid', label: 'Europe/Madrid' },
      { value: 'Europe/Berlin', label: 'Europe/Berlin' },
      { value: 'Asia/Tokyo', label: 'Asia/Tokyo' },
    ]

const DAY_OPTIONS = [
  { value: '0', label: 'Sunday' },
  { value: '1', label: 'Monday' },
  { value: '6', label: 'Saturday' },
]

const FORMAT_OPTIONS = [
  { value: 'markdown', label: 'Markdown' },
  { value: 'plain', label: 'Plain text' },
  { value: 'org', label: 'Org mode' },
]

// ─── SettingsPage ──────────────────────────────────────────────

export function SettingsPage() {
  const [settings, setSettings] = useState<UserSettings | null>(null)
  const [formats, setFormats] = useState<DateFormatOption[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const navigate = useNavigate()
  const { openTab } = useTabs()

  // Auto-open tab for settings view
  useEffect(() => {
    openTab({ name: 'settings', type: 'settings', title: 'Settings', params: {} })
  }, [openTab])

  useEffect(() => {
    let cancelled = false

    async function load() {
      try {
        const [userSettings, dateFormats] = await Promise.all([
          api.getSettings(),
          api.getDateFormats(),
        ])
        if (!cancelled) {
          setSettings(userSettings)
          setFormats(dateFormats)
          setLoading(false)
        }
      } catch (err) {
        if (!cancelled) {
          toast.error(
            `Failed to load settings: ${err instanceof Error ? err.message : 'Unknown error'}`,
          )
          setLoading(false)
        }
      }
    }

    load()
    return () => {
      cancelled = true
    }
  }, [])

  const handleSave = useCallback(async () => {
    if (!settings) return
    setSaving(true)

    try {
      const updated = await api.updateSettings({
        timezone: settings.timezone,
        journalFormat: settings.journalFormat,
        startOfWeek: settings.startOfWeek,
        preferredFormat: settings.preferredFormat,
      })
      setSettings(updated)
      toast.success('Settings saved')
    } catch (err) {
      toast.error(
        `Failed to save settings: ${err instanceof Error ? err.message : 'Unknown error'}`,
      )
    } finally {
      setSaving(false)
    }
  }, [settings])

  function updateSetting<K extends keyof UserSettings>(key: K, value: UserSettings[K]) {
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev))
  }

  if (loading) {
    return (
      <div>
        <div
          style={{
            height: '28px',
            width: '160px',
            background: 'var(--color-surface-subtle)',
            borderRadius: 'var(--radius-sm)',
            marginBottom: 'var(--space-6)',
            animation: 'pulse 1.5s ease-in-out infinite',
          }}
        />
        <div
          style={{
            height: '200px',
            background: 'var(--color-surface-subtle)',
            borderRadius: 'var(--radius-lg)',
            animation: 'pulse 1.5s ease-in-out infinite',
          }}
        />
      </div>
    )
  }

  if (!settings) {
    return (
      <div style={{ textAlign: 'center', padding: 'var(--space-12) var(--space-4)' }}>
        <p style={{ fontSize: '14px', color: 'var(--color-danger)', fontWeight: 600 }}>
          Failed to load settings
        </p>
      </div>
    )
  }

  return (
    <ErrorBoundary>
    <div style={{ maxWidth: '640px' }}>
      {/* Back + title */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-3)',
          marginBottom: 'var(--space-6)',
        }}
      >
        <button
          onClick={() => navigate({ to: '/' })}
          aria-label="Back to home"
          title="Back to home"
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--color-text-muted)',
            padding: 'var(--space-1)',
            borderRadius: 'var(--radius-md)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
          className="topbar-action"
        >
          <ArrowLeft size={18} />
        </button>
        <h1
          style={{
            fontSize: '28px',
            fontWeight: 700,
            color: 'var(--color-text-primary)',
          }}
        >
          Settings
        </h1>
      </div>

      {/* Journal settings */}
      <SettingsCard title="Journal">
        <FormField label="Date format" hint="How dates appear in journal entries">
          <Select
            value={settings.journalFormat}
            onChange={(val) => updateSetting('journalFormat', val)}
            options={formats.map((f) => ({ value: f.pattern, label: f.example }))}
            placeholder="Select date format…"
          />
        </FormField>

        <FormField label="Timezone">
          <Select
            value={settings.timezone}
            onChange={(val) => updateSetting('timezone', val)}
            options={TIMEZONE_OPTIONS}
          />
        </FormField>

        <FormField label="Start of week">
          <Select
            value={String(settings.startOfWeek)}
            onChange={(val) => updateSetting('startOfWeek', Number(val))}
            options={DAY_OPTIONS}
          />
        </FormField>
      </SettingsCard>

      {/* Editor settings */}
      <SettingsCard title="Editor">
        <FormField label="Preferred format" hint="Default format for new blocks">
          <Select
            value={settings.preferredFormat}
            onChange={(val) => updateSetting('preferredFormat', val)}
            options={FORMAT_OPTIONS}
          />
        </FormField>
      </SettingsCard>

      {/* Save button */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'flex-end',
          paddingTop: 'var(--space-2)',
        }}
      >
        <button
          onClick={handleSave}
          disabled={saving}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-2)',
            padding: '10px var(--space-5)',
            borderRadius: 'var(--radius-md)',
            border: 'none',
            background: 'var(--color-primary)',
            color: 'var(--color-on-primary)',
            fontSize: '14px',
            fontWeight: 600,
            cursor: saving ? 'not-allowed' : 'pointer',
            opacity: saving ? 0.6 : 1,
            transition: 'opacity var(--motion-fast) var(--ease-standard), background var(--motion-fast) var(--ease-standard)',
            fontFamily: 'inherit',
          }}
          className="btn-primary"
        >
          <Save size={16} />
          {saving ? 'Saving…' : 'Save settings'}
        </button>
      </div>
    </div>
    </ErrorBoundary>
  )
}
