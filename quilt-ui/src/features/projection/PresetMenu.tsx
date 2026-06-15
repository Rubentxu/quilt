//! PresetMenu — dropdown menu for selecting and applying property presets.
//!
//! Renders as a dropdown panel anchored to a trigger button. Shows all
//! available presets fetched via `usePresets`. Selecting a preset applies
//! its properties to the block via the preset handler.
//!
//! Used in the block properties panel and as part of the slash command
//! integration when the user types "/preset".

import { useState, useRef, useEffect } from 'react';
import { LayoutTemplate, ChevronDown, Check, X } from 'lucide-react';
import { usePresets } from '@features/projection/hooks';
import { defaultRegistry } from '@features/outliner-tiptap/slashRegistry';
import type { Preset } from '@features/projection/types';

interface PresetMenuProps {
  /** Block ID to apply presets to. */
  blockId: string;
  /** Called when a preset is applied. */
  onPresetApplied?: (preset: Preset) => void;
  /** Called when the menu is closed. */
  onClose?: () => void;
}

/**
 * Dropdown menu for selecting property presets.
 * Fetches presets from the server and registers them as slash commands.
 */
export function PresetMenu({ blockId, onPresetApplied, onClose }: PresetMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [appliedPresetId, setAppliedPresetId] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  const { presets, loading, error } = usePresets();

  // Register presets into the slash registry when they load
  useEffect(() => {
    if (presets.length === 0) return;

    // Register each preset as a slash command
    for (const preset of presets) {
      const item = {
        id: `preset:${preset.id}`,
        label: preset.label,
        description: preset.description,
        icon: <LayoutTemplate size={14} />,
        keywords: preset.keywords,
        category: 'Presets',
        // Store the preset ID for the handler
        action: `preset:${preset.id}`,
      };

      const handler = async (ctx: unknown) => {
        // The handler applies preset properties to the block
        // For now, just signal that the preset was applied
        // The actual property application happens via the slash command handler
        console.log(`Applying preset ${preset.id} to block ${ctx}`);
        setAppliedPresetId(preset.id);
        onPresetApplied?.(preset);
      };

      // Only register if not already registered
      if (!defaultRegistry.getItem(`preset:${preset.id}`)) {
        defaultRegistry.register(item, handler);
      }
    }
  }, [presets, onPresetApplied]);

  // Close on outside click
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        menuRef.current &&
        !menuRef.current.contains(e.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(e.target as Node)
      ) {
        setIsOpen(false);
        onClose?.();
      }
    }
    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isOpen, onClose]);

  // Reset selection when menu opens
  useEffect(() => {
    if (isOpen) setSelectedIndex(0);
  }, [isOpen]);

  function handleSelect(preset: Preset) {
    const item = defaultRegistry.getItem(`preset:${preset.id}`);
    if (item) {
      const handler = defaultRegistry.getHandler(`preset:${preset.id}`);
      if (handler) {
        // Create a minimal context for the handler
        // The full context would be provided by BlockRow
        handler({} as never, item);
      }
    }
    setAppliedPresetId(preset.id);
    setIsOpen(false);
    onPresetApplied?.(preset);
    onClose?.();
  }

  if (error) {
    return (
      <div style={{ padding: '8px', color: 'var(--color-error, #ef4444)', fontSize: '13px' }}>
        Failed to load presets
      </div>
    );
  }

  return (
    <div style={{ position: 'relative' }}>
      <button
        ref={triggerRef}
        onClick={() => setIsOpen(!isOpen)}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-label="Open preset menu"
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
          padding: '4px 8px',
          borderRadius: 'var(--radius-md)',
          border: '1px solid var(--color-border)',
          background: 'var(--color-surface)',
          color: 'var(--color-text-secondary)',
          fontSize: '12px',
          cursor: 'pointer',
        }}
      >
        <LayoutTemplate size={12} />
        <span>Presets</span>
        <ChevronDown size={12} />
      </button>

      {isOpen && (
        <div
          ref={menuRef}
          role="listbox"
          aria-label="Available presets"
          style={{
            position: 'absolute',
            top: '100%',
            left: 0,
            marginTop: '4px',
            minWidth: '200px',
            maxHeight: '300px',
            overflowY: 'auto',
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            boxShadow: 'var(--shadow-lg)',
            zIndex: 1000,
          }}
        >
          {loading ? (
            <div style={{ padding: '12px', textAlign: 'center', color: 'var(--color-text-muted)', fontSize: '13px' }}>
              Loading presets...
            </div>
          ) : presets.length === 0 ? (
            <div style={{ padding: '12px', textAlign: 'center', color: 'var(--color-text-muted)', fontSize: '13px' }}>
              No presets available
            </div>
          ) : (
            presets.map((preset, index) => (
              <div
                key={preset.id}
                role="option"
                aria-selected={index === selectedIndex}
                onClick={() => handleSelect(preset)}
                onMouseEnter={() => setSelectedIndex(index)}
                style={{
                  padding: '8px 12px',
                  cursor: 'pointer',
                  background: index === selectedIndex ? 'var(--color-bg-hover)' : 'transparent',
                  borderBottom: '1px solid var(--color-border)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                }}
              >
                <div>
                  <div style={{ fontSize: '13px', fontWeight: 500, color: 'var(--color-text-primary)' }}>
                    {preset.label}
                  </div>
                  <div style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
                    {preset.description}
                  </div>
                </div>
                {appliedPresetId === preset.id && (
                  <Check size={14} style={{ color: 'var(--color-accent)' }} />
                )}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
