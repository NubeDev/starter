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
  label: string
  href: string
  icon: LucideIcon
  badge?: string
}

export interface NavGroup {
  title: string
  items: NavItem[]
}

export const NAV_GROUPS: NavGroup[] = [
  {
    title: 'Overview',
    items: [
      { label: 'Home',      href: '/',          icon: Home },
      { label: 'Dashboard', href: '/dashboard', icon: Gauge, badge: 'live' },
    ],
  },
  {
    title: 'Fleet',
    items: [
      { label: 'Devices',  href: '#devices',  icon: Cpu,      badge: '412' },
      { label: 'Flows',    href: '#flows',    icon: Boxes },
      { label: 'Activity', href: '#activity', icon: Activity, badge: '3.4k' },
    ],
  },
  {
    title: 'Platform',
    items: [
      { label: 'Extensions', href: '#extensions', icon: Layers,    badge: '7' },
      { label: 'Insights',   href: '#insights',   icon: Sparkles },
      { label: 'Git ops',    href: '#git',        icon: GitBranch },
      { label: 'Settings',   href: '/settings',   icon: Settings },
    ],
  },
]
