import * as React from "react"
import { IconSitemap } from "@tabler/icons-react"

import { NavMain, type NavMainItem } from "@/components/nav-main"
import { NavUser, type NavUserProps } from "@/components/nav-user"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"

export interface AppSidebarProps extends React.ComponentProps<typeof Sidebar> {
  navMain: NavMainItem[]
  activeUrl?: string
  user: NavUserProps["user"]
  onLogout?: () => void
  brand?: { title: string; url: string }
  onNavigate?: (url: string) => void
  extraContent?: React.ReactNode
}

export function AppSidebar({
  navMain,
  activeUrl,
  user,
  onLogout,
  brand = { title: "flow-agent", url: "/flows" },
  onNavigate,
  extraContent,
  ...props
}: AppSidebarProps) {
  const handleBrand = (e: React.MouseEvent<HTMLAnchorElement>) => {
    if (!onNavigate) return
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.button === 1) return
    e.preventDefault()
    onNavigate(brand.url)
  }
  return (
    <Sidebar collapsible="offcanvas" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              className="data-[slot=sidebar-menu-button]:p-1.5!"
            >
              <a href={brand.url} onClick={handleBrand}>
                <IconSitemap
                  className="size-5!"
                  style={{ color: "var(--accent-flows)" }}
                />
                <span className="text-base font-semibold">{brand.title}</span>
              </a>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={navMain} activeUrl={activeUrl} onNavigate={onNavigate} />
        {extraContent}
      </SidebarContent>
      <SidebarFooter className="pb-10">
        <NavUser user={user} onLogout={onLogout} />
      </SidebarFooter>
    </Sidebar>
  )
}
