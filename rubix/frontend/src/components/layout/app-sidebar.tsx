import { useLayout } from '@nube/starter-ui-core/layout'
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
  // Live composition: static NAV_GROUPS + a dynamic Dashboards group
  // fed by the rubix-agent SSE feed (see
  // `rubix/docs/scope/dashboards/09-live-sidebar-sse.md`).
  const navGroups = toSidebarNavGroups(useNavGroups())
  return (
    <Sidebar collapsible={collapsible} variant={variant}>
      <SidebarHeader>
        <TeamSwitcher teams={sidebarData.teams} />
      </SidebarHeader>
      <SidebarContent>
        {navGroups.map((props) => (
          <NavGroup key={props.title} {...props} />
        ))}
      </SidebarContent>
      <SidebarFooter>
        <NavUser user={sidebarData.user} />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  )
}
