//! Projection types — mirror the Rust `ProjectionView` domain type.
//!
//! These types represent the visual output of the projection resolution
//! process: text content, links, decorations, conflicts, and effective
//! properties. The UI consumes these types to render blocks.

import type { BlockProperty } from '@shared/types/api';

// ─── Link kinds ────────────────────────────────────────────────────────────────

/** Kind of link — determines how the UI renders the link affordance. */
export type LinkKind = 'external' | 'media' | 'page-ref' | 'block-ref';

/** A link extracted or derived from a block property. */
export interface LinkView {
  /** URL or identifier of the link. */
  url: string;
  /** Human-readable label (may be empty). */
  label: string;
  /** Kind of link. */
  kind: LinkKind;
}

// ─── Decoration kinds ───────────────────────────────────────────────────────

/** Kind of decoration — visual annotation applied by a projection contract. */
export type DecorationKind =
  | 'task-checkbox'
  | 'status-badge'
  | 'media-preview'
  | 'heading-anchor'
  | 'date-indicator'
  | 'link-affordance'
  | 'generic-badge';

/** A visual decoration produced by a projection contract. */
export interface Decoration {
  /** What kind of decoration this is. */
  kind: DecorationKind;
  /** Property key this decoration targets (e.g. "status", "deadline"). */
  target: string;
  /** The property value driving this decoration. */
  value: string | number | boolean | null;
  /** Higher weight = rendered more prominently. Range 0–255. */
  weight: number;
}

// ─── Conflict ───────────────────────────────────────────────────────────────

/** A conflict arising from the projection resolution algorithm. */
export interface ProjectionConflict {
  /** Human-readable reason for the conflict. */
  reason: string;
  /** IDs of all contracts that tied in score/priority. */
  candidates: string[];
  /** The winning contract ID, if one could be determined. */
  winner: string | null;
  /** The block ID this conflict pertains to. */
  blockId: string;
}

// ─── Projection view ─────────────────────────────────────────────────────────

/**
 * The complete visual projection of a block.
 *
 * Produced by `GET /api/v1/blocks/:id/projection`. All fields are
 * public for convenient access in the UI layer.
 */
export interface ProjectionView {
  /** Raw text content from the block. */
  text: string;
  /** Links extracted or derived from block properties. */
  links: LinkView[];
  /** Child block IDs (preserved in order). */
  children: string[];
  /** Visual decorations from active contracts. */
  decorations: Decoration[];
  /** Conflicts from ambiguous resolution. */
  conflicts: ProjectionConflict[];
  /** Effective properties for the view (base + derived). */
  properties: Record<string, string | number | boolean | null>;
}

// ─── Preset types ───────────────────────────────────────────────────────────

/** A property preset that can be applied to a block. */
export interface Preset {
  /** Preset identifier (e.g., "/TODO"). */
  id: string;
  /** Human-readable label derived from the preset id. */
  label: string;
  /** Human-readable description. */
  description: string;
  /** Required argument kinds for this preset. */
  requiredArgs: PresetArgKind[];
  /** Keywords for search. */
  keywords: string[];
}

/** Kind of argument required by a preset. */
export type PresetArgKind = 'date' | 'url' | 'text';

/** Response from `GET /api/v1/presets`. */
export interface PresetListResponse {
  presets: Preset[];
  count: number;
}

// ─── Property visibility & mutability ───────────────────────────────────────

/**
 * Property visibility — controls whether a property is shown in the panel.
 * Mirrors `quilt_domain::properties::PropertyVisibility`.
 */
export type PropertyVisibility = 'visible' | 'hidden' | 'derived';

/**
 * Property mutability — controls whether a property can be edited.
 * Mirrors `quilt_domain::properties::PropertyMutability`.
 */
export type PropertyMutability = 'editable' | 'derived' | 'immutable';

/** Extended property with visibility and mutability metadata. */
export interface PropertyWithMeta extends BlockProperty {
  visibility: PropertyVisibility;
  mutability: PropertyMutability;
}
