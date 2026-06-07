import {
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
} from '@tanstack/react-router'
import { AppShell } from './shared/components/AppShell'

// ─── Lazy route components ────────────────────────────────────
// Each route is loaded on demand via dynamic import. The bundler emits
// one chunk per page so the initial download only contains the shell.
//
// `lazyRouteComponent` wires up `preload()` semantics and survives
// stale chunk URLs after a redeploy (it retries the import).
const HomePage = lazyRouteComponent(() => import('@pages/HomePage'), 'HomePage')
const PageViewPage = lazyRouteComponent(
  () => import('@pages/PageViewPage'),
  'PageViewPage',
)
const JournalPage = lazyRouteComponent(
  () => import('@pages/JournalPage'),
  'JournalPage',
)
const SettingsPage = lazyRouteComponent(
  () => import('@pages/SettingsPage'),
  'SettingsPage',
)
const AllPagesPage = lazyRouteComponent(
  () => import('@pages/AllPagesPage'),
  'AllPagesPage',
)
const GraphViewPage = lazyRouteComponent(
  () => import('@pages/GraphViewPage'),
  'GraphViewPage',
)
const TablePage = lazyRouteComponent(
  () => import('@pages/TablePage'),
  'TablePage',
)
const KanbanPage = lazyRouteComponent(
  () => import('@pages/KanbanPage'),
  'KanbanPage',
)
const DashboardPage = lazyRouteComponent(
  () => import('@pages/DashboardPage'),
  'DashboardPage',
)

// ─── Route tree ─────────────────────────────────────────────────
const rootRoute = createRootRoute({
  component: AppShell,
})

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: HomePage,
})

const pageRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/page/$name',
  component: PageViewPage,
})

const journalRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/journal/$date',
  component: JournalPage,
})

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsPage,
})

const pagesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/pages',
  component: AllPagesPage,
})

const graphRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/graph',
  component: GraphViewPage,
})

const tableRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/table',
  component: TablePage,
})

const kanbanRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/kanban',
  component: KanbanPage,
})

const dashboardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/dashboard',
  component: DashboardPage,
})

const routeTree = rootRoute.addChildren([
  homeRoute,
  pageRoute,
  journalRoute,
  settingsRoute,
  pagesRoute,
  graphRoute,
  tableRoute,
  kanbanRoute,
  dashboardRoute,
])

// ─── Router ──────────────────────────────────────────────────────
// `defaultPreload: 'intent'` triggers a chunk preload on Link hover/focus.
// That way the user sees the route component almost instantly when they
// click, while still only paying for the bytes they actually visit.
export const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
  // Don't block navigation while the next chunk is in flight.
  defaultPreloadStaleTime: 30_000,
})

// ─── Type registration ───────────────────────────────────────────
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
