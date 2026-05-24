import { Sparkles } from 'lucide-react'
import { NAV_GROUPS } from '@/lib/nav'
import { type SidebarData } from '../types'

// `title` fields here are i18n message keys — the sidebar components
// (nav-group, team-switcher) translate them at render time via useIntl.
export const sidebarData: SidebarData = {
  user: {
    name: 'Operator',
    email: 'ops@nube-io.com',
    avatar: '',
  },
  teams: [
    {
      name: 'brand.name',
      logo: Sparkles,
      plan: 'brand.tagline',
    },
  ],
  navGroups: NAV_GROUPS.map((g) => ({
    title: g.titleKey,
    items: g.items.map((item) => ({
      title: item.labelKey,
      url: item.href,
      icon: item.icon,
      badge: item.badge,
    })),
  })),
}
