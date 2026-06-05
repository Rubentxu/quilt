// ─── Sidebar sections barrel (PR 1+2+3) ───────────────────────────
//
// Centralised re-exports for the sub-components living under
// `features/sidebar/sections/`. Keeps the rest of the sidebar
// (Sidebar.tsx, future test files) free of deep relative paths
// and makes the public surface of the sections folder explicit.

export { GroupHeader } from './GroupHeader'
export { SidebarItem } from './SidebarItem'
export { SidebarSkeleton } from './SidebarSkeleton'
export { TemplateSection } from './TemplateSection'
export { RecentsSection } from './RecentsSection'
