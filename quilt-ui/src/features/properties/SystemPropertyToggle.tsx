//! SystemPropertyToggle — show/hide toggle for system/internal properties.
//!
//! System properties (prefixed with `_` or known system keys like
//! `created_at`, `updated_at`, `id`) are hidden by default in the
//! BlockPropertiesPanel to reduce noise. This toggle lets power users
//! reveal them when needed.

import { useState } from 'react';
import { Eye, EyeOff } from 'lucide-react';

interface SystemPropertyToggleProps {
  /** Whether system properties are currently visible. */
  showSystem: boolean;
  /** Called when the user toggles visibility. */
  onToggle: (show: boolean) => void;
}

/**
 * Pill-shaped toggle button for system property visibility.
 * Renders with Eye/EyeOff icons and a label.
 */
export function SystemPropertyToggle({ showSystem, onToggle }: SystemPropertyToggleProps) {
  const [isFocused, setIsFocused] = useState(false);

  return (
    <button
      type="button"
      onClick={() => onToggle(!showSystem)}
      onFocus={() => setIsFocused(true)}
      onBlur={() => setIsFocused(false)}
      aria-label={showSystem ? 'Hide system properties' : 'Show system properties'}
      aria-pressed={showSystem}
      title={showSystem ? 'Hide system properties' : 'Show system properties'}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 'var(--space-1, 4px)',
        padding: '2px var(--space-2, 8px)',
        borderRadius: 'var(--radius-full, 9999px)',
        border: '1px solid var(--color-border, #e5e7eb)',
        background: showSystem
          ? 'var(--color-accent, #3b82f6)'
          : 'var(--color-surface, #ffffff)',
        color: showSystem
          ? 'white'
          : 'var(--color-text-muted, #6b7280)',
        fontSize: '11px',
        fontWeight: 500,
        cursor: 'pointer',
        transition: 'all var(--motion-fast, 150ms)',
        outline: isFocused ? '2px solid var(--color-accent, #3b82f6)' : 'none',
        outlineOffset: '1px',
      }}
    >
      {showSystem ? <EyeOff size={12} /> : <Eye size={12} />}
      <span>{showSystem ? 'Hide system' : 'Show system'}</span>
    </button>
  );
}
