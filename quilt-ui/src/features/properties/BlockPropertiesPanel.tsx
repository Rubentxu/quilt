import { useState, useEffect, useCallback } from 'react';
import { Plus, X, Tag, Hash, ToggleLeft, Calendar, Link2, Lock } from 'lucide-react';
import { api } from '@core/api-client';
import type { BlockProperty } from '@shared/types/api';
import { resolveNaturalDate, isDatePropertyKey } from '@shared/utils/naturalDate';
import toast from 'react-hot-toast';
import { SystemPropertyToggle } from './SystemPropertyToggle';
import type { PropertyWithMeta, PropertyVisibility, PropertyMutability } from '@features/projection/types';

// ─── Derived-property badge ─────────────────────────────────────────────────

interface MutabilityBadgeProps {
  mutability: PropertyMutability;
}

function MutabilityBadge({ mutability }: MutabilityBadgeProps) {
  if (mutability === 'editable') return null;
  const label = mutability === 'derived' ? 'derived' : 'immutable';
  return (
    <span
      title={`This property is ${mutability} and cannot be edited directly.`}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '2px',
        fontSize: '10px',
        color: 'var(--color-text-disabled, #9ca3af)',
        padding: '1px 4px',
        borderRadius: 'var(--radius-sm, 4px)',
        background: 'var(--color-bg-subtle, #f3f4f6)',
      }}
    >
      <Lock size={8} />
      {label}
    </span>
  );
}

// ─── Visibility filter ───────────────────────────────────────────────────────

/**
 * Keys that are considered "system" properties and hidden by default.
 */
const SYSTEM_PROPERTY_KEYS = new Set([
  'id',
  'created_at',
  'updated_at',
  'created_by',
  'updated_by',
  'page_id',
  'parent_id',
  'order',
  'level',
  'format',
  'block_type',
  'marker',
  '_id',
  '_created',
  '_updated',
]);

function isSystemProperty(key: string): boolean {
  return key.startsWith('_') || SYSTEM_PROPERTY_KEYS.has(key);
}

function getVisibilityFilter(
  visibility: PropertyVisibility | undefined,
  showSystem: boolean,
): boolean {
  // If no visibility metadata, fall back to key-based detection
  if (visibility === undefined) {
    return showSystem || !isSystemProperty(visibility ?? '');
  }
  switch (visibility) {
    case 'visible':
      return true;
    case 'hidden':
      return false;
    case 'derived':
      return showSystem;
    default:
      return true;
  }
}

// ─── Props ─────────────────────────────────────────────────────────────────

interface BlockPropertiesPanelProps {
  blockId: string;
  onClose: () => void;
  /**
   * Optional rich property list with visibility/mutability metadata.
   * When provided, the panel filters and guards editing based on this metadata.
   * Falls back to fetching via `api.getBlockProperties` when omitted.
   */
  propertiesWithMeta?: PropertyWithMeta[];
  /** Whether the panel is in edit mode (shows save/cancel actions). */
  isEditing?: boolean;
  /** Called when edit mode is toggled. */
  onEditModeChange?: (editing: boolean) => void;
}

const PROPERTY_TYPE_ICONS: Record<string, React.ReactNode> = {
  string: <Tag size={14} />,
  number: <Hash size={14} />,
  boolean: <ToggleLeft size={14} />,
  date: <Calendar size={14} />,
  select: <Tag size={14} />,
  page_ref: <Link2 size={14} />,
};

// ─── Component ──────────────────────────────────────────────────────────────

export function BlockPropertiesPanel({
  blockId,
  onClose,
  propertiesWithMeta,
  isEditing: controlledEditing,
  onEditModeChange,
}: BlockPropertiesPanelProps) {
  const [properties, setProperties] = useState<(BlockProperty | PropertyWithMeta)[]>([]);
  const [loading, setLoading] = useState(true);
  const [newKey, setNewKey] = useState('');
  const [showAddForm, setShowAddForm] = useState(false);
  const [showSystem, setShowSystem] = useState(false);
  // Internal edit mode state, overridable via prop
  const [internalEditing, setInternalEditing] = useState(false);
  const isEditing = controlledEditing ?? internalEditing;

  const setEditing = useCallback(
    (val: boolean) => {
      if (onEditModeChange) {
        onEditModeChange(val);
      } else {
        setInternalEditing(val);
      }
    },
    [onEditModeChange],
  );

  useEffect(() => {
    loadProperties();
  }, [blockId]);

  async function loadProperties() {
    try {
      if (propertiesWithMeta) {
        setProperties(propertiesWithMeta);
      } else {
        const props = await api.getBlockProperties(blockId);
        setProperties(props);
      }
    } catch {
      // Properties endpoint may not exist yet on the backend
      setProperties([]);
    } finally {
      setLoading(false);
    }
  }

  async function updateProperty(key: string, value: unknown) {
    // Guard: check mutability before attempting update
    const prop = properties.find(p => p.key === key);
    if (prop && 'mutability' in prop) {
      const mutability = (prop as PropertyWithMeta).mutability;
      if (mutability !== 'editable') {
        toast.error(`"${key}" is ${mutability} and cannot be edited.`);
        return;
      }
    }

    // NL Dates V1: when the user types a natural-language date
    // ("today" / "tomorrow" / "yesterday") in a date-typed property
    // (or one of the canonical date keys — `deadline`, `scheduled`,
    // `date`), resolve it to a real ISO YYYY-MM-DD string before
    // persisting. Keeps the backend date-agnostic and matches the
    // behaviour of `BlockRow.saveToApi`.
    let resolved: unknown = value;
    if (typeof value === 'string' && isDatePropertyKey(key)) {
      const r = resolveNaturalDate(value);
      if (r !== null) resolved = r;
    }
    try {
      await api.setBlockProperty(blockId, key, resolved);
      setProperties(prev =>
        prev.map(p =>
          p.key === key
            ? { ...p, value: resolved as string | number | boolean | null }
            : p,
        ),
      );
    } catch {
      toast.error('Failed to update property');
    }
  }

  async function deleteProperty(key: string) {
    // Guard: check mutability before attempting delete
    const prop = properties.find(p => p.key === key);
    if (prop && 'mutability' in prop) {
      const mutability = (prop as PropertyWithMeta).mutability;
      if (mutability !== 'editable') {
        toast.error(`"${key}" is ${mutability} and cannot be deleted.`);
        return;
      }
    }
    try {
      await api.deleteBlockProperty(blockId, key);
      setProperties(prev => prev.filter(p => p.key !== key));
    } catch {
      toast.error('Failed to delete property');
    }
  }

  async function addProperty() {
    if (!newKey.trim()) return;
    try {
      await api.setBlockProperty(blockId, newKey.trim(), '');
      setNewKey('');
      setShowAddForm(false);
      loadProperties();
    } catch {
      toast.error('Failed to add property');
    }
  }

  // Filter properties based on visibility metadata or key patterns
  const visibleProperties = properties.filter(prop => {
    if ('visibility' in prop) {
      return getVisibilityFilter(prop.visibility, showSystem);
    }
    // Fallback: key-based system property detection
    return showSystem || !isSystemProperty(prop.key);
  });

  if (loading) {
    return (
      <div
        style={{
          padding: 'var(--space-4)',
          color: 'var(--color-text-muted)',
          fontSize: '13px',
        }}
      >
        Loading properties...
      </div>
    );
  }

  return (
    <div
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: 'var(--space-3) var(--space-4)',
          borderBottom: '1px solid var(--color-border)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <span
            style={{
              fontSize: '13px',
              fontWeight: 600,
              color: 'var(--color-text-primary)',
            }}
          >
            Properties
          </span>
          <SystemPropertyToggle showSystem={showSystem} onToggle={setShowSystem} />
        </div>
        <button
          onClick={() => setShowAddForm(true)}
          aria-label="Add property"
          title="Add property"
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--color-text-muted)',
            padding: '2px',
            display: 'flex',
            alignItems: 'center',
          }}
        >
          <Plus size={16} />
        </button>
      </div>

      {/* Properties list */}
      {visibleProperties.length === 0 && !showAddForm ? (
        <div
          style={{
            padding: 'var(--space-6) var(--space-4)',
            textAlign: 'center',
            color: 'var(--color-text-muted)',
            fontSize: '13px',
          }}
        >
          No properties yet
        </div>
      ) : (
        <div>
          {visibleProperties.map(prop => {
            // Determine if this property is editable
            const mutability = 'mutability' in prop ? (prop as PropertyWithMeta).mutability : 'editable';
            const isLocked = mutability !== 'editable';

            return (
              <div
                key={prop.key}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-2)',
                  padding: 'var(--space-2) var(--space-4)',
                  borderBottom: '1px solid var(--color-border)',
                  fontSize: '13px',
                  opacity: isLocked && !showSystem ? 0.6 : 1,
                }}
              >
                <span
                  style={{
                    color: 'var(--color-text-muted)',
                    display: 'flex',
                    alignItems: 'center',
                  }}
                >
                  {PROPERTY_TYPE_ICONS[prop.type] || <Tag size={14} />}
                </span>
                <span
                  style={{
                    color: 'var(--color-text-secondary)',
                    minWidth: '100px',
                    fontWeight: 500,
                  }}
                >
                  {prop.key}
                </span>
                <MutabilityBadge mutability={mutability} />
                {isLocked ? (
                  <span
                    style={{
                      flex: 1,
                      color: 'var(--color-text-disabled)',
                      fontSize: '12px',
                      fontStyle: 'italic',
                    }}
                  >
                    {String(prop.value ?? '')}
                  </span>
                ) : prop.type === 'boolean' ? (
                  <input
                    type="checkbox"
                    checked={!!prop.value}
                    onChange={e => updateProperty(prop.key, e.target.checked)}
                    style={{ cursor: 'pointer' }}
                  />
                ) : (
                  <input
                    type={prop.type === 'number' ? 'number' : 'text'}
                    value={String(prop.value ?? '')}
                    onChange={e => {
                      const val =
                        prop.type === 'number'
                          ? Number(e.target.value)
                          : e.target.value;
                      updateProperty(prop.key, val);
                    }}
                    style={{
                      flex: 1,
                      border: 'none',
                      outline: 'none',
                      background: 'transparent',
                      color: 'var(--color-text-primary)',
                      fontSize: '13px',
                      padding: '2px 0',
                      fontFamily: 'inherit',
                    }}
                  />
                )}
                {!isLocked && (
                  <button
                    onClick={() => deleteProperty(prop.key)}
                    aria-label={`Delete property ${prop.key}`}
                    title={`Delete ${prop.key}`}
                    style={{
                      background: 'none',
                      border: 'none',
                      cursor: 'pointer',
                      color: 'var(--color-text-disabled)',
                      padding: '2px',
                      display: 'flex',
                      alignItems: 'center',
                      opacity: 0.5,
                      transition: 'opacity var(--motion-fast)',
                    }}
                    onMouseEnter={e => {
                      (e.currentTarget as HTMLButtonElement).style.opacity = '1';
                    }}
                    onMouseLeave={e => {
                      (e.currentTarget as HTMLButtonElement).style.opacity = '0.5';
                    }}
                  >
                    <X size={12} />
                  </button>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Add property form */}
      {showAddForm && (
        <div
          style={{
            display: 'flex',
            gap: 'var(--space-2)',
            padding: 'var(--space-3) var(--space-4)',
            borderTop: '1px solid var(--color-border)',
          }}
        >
          <input
            type="text"
            value={newKey}
            onChange={e => setNewKey(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') addProperty();
              if (e.key === 'Escape') {
                setShowAddForm(false);
                setNewKey('');
              }
            }}
            placeholder="Property name"
            autoFocus
            style={{
              flex: 1,
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: 'var(--space-1) var(--space-2)',
              fontSize: '13px',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              outline: 'none',
              fontFamily: 'inherit',
            }}
          />
          <button
            onClick={addProperty}
            style={{
              padding: 'var(--space-1) var(--space-3)',
              background: 'var(--color-accent)',
              color: 'white',
              border: 'none',
              borderRadius: 'var(--radius-sm)',
              fontSize: '13px',
              cursor: 'pointer',
            }}
          >
            Add
          </button>
        </div>
      )}
    </div>
  );
}
