import { Sparkles } from 'lucide-react'
import { NAV_GROUPS, type NavGroup as RubixNavGroup } from '@/lib/nav'
import { type SidebarData, type NavGroup as NavGroupShape } from '../types'

// `title` fields here are i18n message keys — the sidebar components
// (nav-group, team-switcher) translate them at render time via useIntl.

/** Project rubix-side `NavGroup`s (label keys + lucide icons) into
 *  the shape `<NavGroup>` consumes. Used by both the static seed
 *  below and the dynamic `useNavGroups()` hook (see `lib/use-nav-groups.ts`). */
export function toSidebarNavGroups(groups: RubixNavGroup[]): NavGroupShape[] {
  return groups.map((g) => ({
    title: g.titleKey,
    items: g.items.map((item) => ({
      title: item.labelKey,
      defaultMessage: item.defaultMessage,
      url: item.href,
      icon: item.icon,
      badge: item.badge,
    })),
  }))
}

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
  navGroups: toSidebarNavGroups(NAV_GROUPS),
}
