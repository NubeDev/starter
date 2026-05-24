import {
  Activity,
  Boxes,
  Cpu,
  Gauge,
  GitBranch,
  Home,
  Layers,
  Settings,
  Sparkles,
  type LucideIcon,
} from 'lucide-react'

export interface NavItem {
  /** i18n message key — resolve via `useIntl().formatMessage({ id: labelKey })`. */
  labelKey: string
  href: string
  icon: LucideIcon
  badge?: string
}

export interface NavGroup {
  /** i18n message key for the group heading. */
  titleKey: string
  items: NavItem[]
}

export const NAV_GROUPS: NavGroup[] = [
  {
    titleKey: 'nav.group.overview',
    items: [
      { labelKey: 'nav.item.home',      href: '/',          icon: Home },
      { labelKey: 'nav.item.dashboard', href: '/dashboard', icon: Gauge, badge: 'live' },
    ],
  },
  {
    titleKey: 'nav.group.fleet',
    items: [
      { labelKey: 'nav.item.devices',  href: '#devices',  icon: Cpu,      badge: '412' },
      { labelKey: 'nav.item.flows',    href: '/flow',     icon: Boxes },
      { labelKey: 'nav.item.activity', href: '#activity', icon: Activity, badge: '3.4k' },
    ],
  },
  {
    titleKey: 'nav.group.platform',
    items: [
      { labelKey: 'nav.item.extensions', href: '#extensions', icon: Layers,    badge: '7' },
      { labelKey: 'nav.item.insights',   href: '#insights',   icon: Sparkles },
      { labelKey: 'nav.item.gitOps',     href: '#git',        icon: GitBranch },
      { labelKey: 'nav.item.settings',   href: '/settings',   icon: Settings },
    ],
  },
]
