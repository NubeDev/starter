import { Sparkles } from 'lucide-react'
import { NAV_GROUPS } from '@/lib/nav'
import { type SidebarData } from '../types'

export const sidebarData: SidebarData = {
  user: {
    name: 'Operator',
    email: 'ops@nube-io.com',
    avatar: '',
  },
  teams: [
    {
      name: 'Nube',
      logo: Sparkles,
      plan: 'IoT Console',
    },
  ],
  navGroups: NAV_GROUPS.map((g) => ({
    title: g.title,
    items: g.items.map((item) => ({
      title: item.label,
      url: item.href,
      icon: item.icon,
      badge: item.badge,
    })),
  })),
}
