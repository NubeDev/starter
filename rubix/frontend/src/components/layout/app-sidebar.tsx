import { useLayout } from '@nube/starter-ui-core/layout'
import { ExtensionSlot } from '@nube/starter-ext-ui'
import { useRouter } from '@tanstack/react-router'
import { type MouseEvent } from 'react'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from '@/components/ui/sidebar'
import { sidebarData, toSidebarNavGroups } from './data/sidebar-data'
import { NavGroup } from './nav-group'
import { NavUser } from './nav-user'
import { TeamSwitcher } from './team-switcher'
import { useNavGroups } from '@/lib/use-nav-groups'

export function AppSidebar() {
  const { collapsible, variant } = useLayout()
  const router = useRouter()
  // Live composition: static NAV_GROUPS + a dynamic Dashboards group
  // fed by the rubix-agent SSE feed (see
  // `rubix/docs/scope/dashboards/09-live-sidebar-sse.md`).
  const navGroups = toSidebarNavGroups(useNavGroups())

  // Extension nav-trees are rendered by remote bundles that can't import
  // the host's router, so their links are plain <a> and trigger full
  // page reloads. Intercept same-origin internal clicks and hand them
  // to TanStack Router for SPA navigation.
  const interceptExtensionNav = (e: MouseEvent<HTMLDivElement>) => {
    if (e.defaultPrevented) return
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return
    const anchor = (e.target as HTMLElement).closest('a')
    if (!anchor) return
    const href = anchor.getAttribute('href')
    if (!href || href.startsWith('http') || href.startsWith('//') || href.startsWith('#')) return
    if (anchor.target && anchor.target !== '_self') return
    e.preventDefault()
    router.navigate({ to: href })
  }

  return (
    <Sidebar collapsible={collapsible} variant={variant}>
      <SidebarHeader>
        <TeamSwitcher teams={sidebarData.teams} />
      </SidebarHeader>
      <SidebarContent>
        {navGroups.map((props) => (
          <NavGroup key={props.title} {...props} />
        ))}
        {/* Nav-tree contributions: extensions declare `slot: sidebar-nav`
            and render their own `Extension -> tab -> nested tab` tree.
            Sits alongside the static/live NavGroups above. */}
        <div onClick={interceptExtensionNav}>
          <ExtensionSlot id="sidebar-nav" />
        </div>
        {/* Compact status-panel slot for `slot: sidebar` contributions. */}
        <ExtensionSlot id="sidebar" />
      </SidebarContent>
      <SidebarFooter>
        <NavUser user={sidebarData.user} />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  )
}
