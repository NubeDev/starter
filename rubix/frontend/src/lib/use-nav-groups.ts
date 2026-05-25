// `useNavGroups` — composes the static `NAV_GROUPS` from `lib/nav.ts`
// with a dynamic **Dashboards** group fed by `useDashboardSidebar()`
// from `@nube/rubix-client-react`. Returned as a fresh array on
// each change so the sidebar re-renders on every snapshot or delta.
//
// Per `rubix/docs/scope/dashboards/09-live-sidebar-sse.md` the
// dynamic group sits between the static **Overview** and **Fleet**
// groups so the operator sees their own dashboards immediately
// under the always-on `/dashboard` link.

import { useMemo } from 'react'
import { Gauge, LayoutDashboard, MessageSquare } from 'lucide-react'
import { useDashboardSidebar } from '@nube/rubix-client-react'

import { NAV_GROUPS, type NavGroup, type NavItem } from './nav'

/** Title shown when the live tail is reconnecting; the dashboards
 *  list is kept visible so a transient drop does not flash. */
const RECONNECTING_BADGE = '…'

export function useNavGroups(): NavGroup[] {
  const sidebar = useDashboardSidebar()

  return useMemo(() => {
    const dashboardGroup = buildDashboardGroup(
      sidebar.items.map((it) => ({
        page_id: it.page_id,
        title: it.title,
      })),
      sidebar.status === 'reconnecting',
    )

    // Insert the dynamic group right after the static Overview
    // group (index 0). Falls back to "prepend" if NAV_GROUPS ever
    // shrinks to nothing.
    const out = [...NAV_GROUPS]
    const insertAt = Math.min(1, out.length)
    out.splice(insertAt, 0, dashboardGroup)
    return out
  }, [sidebar.items, sidebar.status])
}

interface SidebarEntry {
  page_id: string
  title: string
}

function buildDashboardGroup(
  entries: SidebarEntry[],
  reconnecting: boolean,
): NavGroup {
  if (entries.length === 0) {
    // Empty state — surface a single CTA pointing at the chat
    // route so a fresh operator can ask the dashboard assistant to
    // author the first page.
    return {
      titleKey: 'nav.group.dashboards',
      items: [
        {
          labelKey: 'nav.item.createFirstDashboard',
          href: '/chat',
          icon: MessageSquare,
          badge: reconnecting ? RECONNECTING_BADGE : undefined,
        },
      ],
    }
  }

  const items: NavItem[] = entries.map((entry, idx) => ({
    labelKey: `dashboards.sidebar.${entry.page_id}`,
    defaultMessage: entry.title || entry.page_id,
    href: `/dashboards/${encodeURIComponent(
      entry.page_id.replace(/^dashboard\./, ''),
    )}`,
    icon: idx === 0 ? LayoutDashboard : Gauge,
    // Only flag reconnect state on the first entry to avoid a
    // sea of dots; the badge ships only when the operator needs
    // to know the feed is recovering.
    badge: idx === 0 && reconnecting ? RECONNECTING_BADGE : undefined,
  }))

  return {
    titleKey: 'nav.group.dashboards',
    items,
  }
}
