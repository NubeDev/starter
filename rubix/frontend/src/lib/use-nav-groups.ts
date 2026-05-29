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
import { Building2, Gauge, LayoutDashboard, MessageSquare } from 'lucide-react'
import { useDashboardSidebar } from '@nube/rubix-client-react'
import { useTenants } from '@nube/starter-ui-authz'

import { NAV_GROUPS, type NavGroup, type NavItem } from './nav'

/** Title shown when the live tail is reconnecting; the dashboards
 *  list is kept visible so a transient drop does not flash. */
const RECONNECTING_BADGE = '…'

export function useNavGroups(): NavGroup[] {
  const sidebar = useDashboardSidebar()
  const tenants = useTenants()

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

    // Expand the Admin / Access entry with live tenants as
    // collapsible sub-items so an operator can deep-link from the
    // sidebar straight into a tenant's Access surface.
    const list = tenants.data ?? []
    if (list.length > 0) {
      const adminIdx = out.findIndex((g) => g.titleKey === 'nav.group.admin')
      if (adminIdx >= 0) {
        const group = out[adminIdx]
        const accessIdx = group.items.findIndex(
          (it) => it.href === '/admin/access',
        )
        if (accessIdx >= 0) {
          const access = group.items[accessIdx]
          const subItems: NavItem[] = list.map((t) => ({
            labelKey: `access.tenant.${t.id}`,
            defaultMessage: t.display_name || t.slug,
            href: `/admin/access/t/${t.slug}`,
            icon: Building2,
          }))
          const expanded: NavItem = {
            labelKey: access.labelKey,
            defaultMessage: access.defaultMessage,
            href: access.href,
            icon: access.icon,
            badge: access.badge,
            children: subItems,
          }
          const items = [...group.items]
          items[accessIdx] = expanded
          out[adminIdx] = { ...group, items }
        }
      }
    }

    return out
  }, [sidebar.items, sidebar.status, tenants.data])
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
