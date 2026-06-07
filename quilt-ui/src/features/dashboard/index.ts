// ─── dashboard feature — public API ──────────────────────────────
//
// Re-exports the DashboardLayout surface. Importers should pull
// from `@features/dashboard` rather than reaching into the
// individual modules so the internal layout can evolve.

export {
  PanelVisibilityProvider,
  usePanelVisibility,
  DEFAULT_PANELS,
  PANEL_LABELS,
  DASHBOARD_STORAGE_KEY,
  DASHBOARD_EVENT,
  dispatchDashboardChange,
  type DashboardEventDetail,
  type PanelVisibilityContextValue,
} from './PanelVisibilityContext'

export {
  PRESETS,
  PRESET_ORDER,
  PRESET_LABELS,
  PANEL_LABELS as PANEL_LABELS_FROM_PRESETS,
  getPreset,
  findClosestPreset,
  type PanelId,
  type PresetId,
} from './presets'

export { LayoutMenu } from './LayoutMenu'
