import {
  Activity,
  Boxes,
  Cpu,
  Database,
  Gauge,
  GitBranch,
  Home,
  Layers,
  Settings,
  ShieldCheck,
  Sparkles,
  Users,
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
      { labelKey: 'nav.item.flows',    href: '/flows',    icon: Boxes },
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
  {
    // Admin section — operator-only surfaces. `access` mounts the
    // starter-ui-authz `<AuthzAdmin>` panel; `users` is the rubix
    // user-management page; `warehouse` is a Phase C stub that will
    // host the ClickHouse + insights admin once that surface lands.
    titleKey: 'nav.group.admin',
    items: [
      { labelKey: 'nav.item.access',    href: '/admin/access',    icon: ShieldCheck },
      { labelKey: 'nav.item.users',     href: '/admin/users',     icon: Users },
      { labelKey: 'nav.item.warehouse', href: '/admin/warehouse', icon: Database },
    ],
  },
]
