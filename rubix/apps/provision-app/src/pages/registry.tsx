import type { ReactNode } from 'react'
import { ScanLine, Cpu, Building2, FileCode2, LayoutDashboard, type LucideIcon } from 'lucide-react'
import { ScanFlow } from '../scan/ScanFlow'
import { Devices } from '../devices/Devices'
import { Sites } from '../sites/Sites'
import { Templates } from '../templates/Templates'
import { PagePreview } from '../preview/PagePreview'

// ─────────────────────────────────────────────────────────────────────────────
// Page registry — single source of truth for the app's screens. App.tsx and
// NavBar render purely by mapping over PAGES. To add a screen, append a PageDef.
// `group: 'secondary'` puts it in the nav's overflow rail.
// ─────────────────────────────────────────────────────────────────────────────

// Shared actions the Shell exposes to pages.
export interface PageActions {
  // jump to another tab
  onNavigate: (tab: Tab) => void
  // open Page preview deep-linked to a freshly-provisioned page
  onPreview: (pageId: string) => void
  // the page id the preview screen should open on (cleared after consumed)
  previewPageId?: string
}

export interface PageDef {
  tab: string
  label: string
  icon: LucideIcon
  group: 'primary' | 'secondary'
  element: (actions: PageActions) => ReactNode
}

export const PAGES: PageDef[] = [
  {
    tab: 'scan',
    label: 'Scan',
    icon: ScanLine,
    group: 'primary',
    element: ({ onPreview }) => <ScanFlow onPreview={onPreview} />,
  },
  {
    tab: 'devices',
    label: 'Devices',
    icon: Cpu,
    group: 'primary',
    element: () => <Devices />,
  },
  {
    tab: 'preview',
    label: 'Preview',
    icon: LayoutDashboard,
    group: 'primary',
    element: ({ previewPageId }) => <PagePreview initialPageId={previewPageId} />,
  },
  {
    tab: 'sites',
    label: 'Sites',
    icon: Building2,
    group: 'primary',
    element: () => <Sites />,
  },
  // ── Secondary (overflow rail) ────────────────────────────────────────────
  {
    tab: 'templates',
    label: 'Templates',
    icon: FileCode2,
    group: 'secondary',
    element: () => <Templates />,
  },
]

export type Tab = string

export const DEFAULT_TAB: Tab = 'scan'

export const primaryPages = () => PAGES.filter((p) => p.group === 'primary')
export const secondaryPages = () => PAGES.filter((p) => p.group === 'secondary')
export const pageByTab = (tab: Tab) => PAGES.find((p) => p.tab === tab)
