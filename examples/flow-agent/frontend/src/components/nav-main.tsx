import type * as React from "react"
import { type Icon } from "@tabler/icons-react"

import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar"

export interface NavMainSubItem {
  title: string
  url: string
  icon?: Icon
  badge?: React.ReactNode
}

export interface NavMainItem {
  title: string
  url: string
  icon?: Icon
  accent?: string
  items?: NavMainSubItem[]
  subTestId?: string
}

export function NavMain({
  items,
  activeUrl,
  onNavigate,
}: {
  items: NavMainItem[]
  activeUrl?: string
  onNavigate?: (url: string) => void
}) {
  const handleClick =
    (url: string) =>
    (e: React.MouseEvent<HTMLAnchorElement>) => {
      if (!onNavigate) return
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.button === 1) return
      e.preventDefault()
      onNavigate(url)
    }
  return (
    <SidebarGroup>
      <SidebarGroupContent className="flex flex-col gap-2">
        <SidebarMenu>
          {items.map((item) => {
            const isSectionActive =
              !!activeUrl &&
              (activeUrl === item.url || activeUrl.startsWith(`${item.url}/`))
            return (
              <SidebarMenuItem key={item.title}>
                <SidebarMenuButton
                  asChild
                  tooltip={item.title}
                  isActive={isSectionActive && !item.items}
                >
                  <a
                    href={item.url}
                    onClick={handleClick(item.url)}
                    aria-current={isSectionActive && !item.items ? "page" : undefined}
                    style={item.accent ? ({ "--nav-accent": item.accent } as React.CSSProperties) : undefined}
                  >
                    {item.icon && (
                      <item.icon className={item.accent ? "text-(--nav-accent)" : undefined} />
                    )}
                    <span>{item.title}</span>
                  </a>
                </SidebarMenuButton>
                {item.items && item.items.length > 0 ? (
                  <SidebarMenuSub data-testid={item.subTestId}>
                    {item.items.map((sub) => {
                      const isSubActive =
                        !!activeUrl &&
                        (activeUrl === sub.url ||
                          activeUrl.startsWith(`${sub.url}/`))
                      return (
                        <SidebarMenuSubItem key={sub.url}>
                          <SidebarMenuSubButton asChild isActive={isSubActive}>
                            <a
                              href={sub.url}
                              onClick={handleClick(sub.url)}
                              aria-current={isSubActive ? "page" : undefined}
                              style={item.accent ? ({ "--nav-accent": item.accent } as React.CSSProperties) : undefined}
                            >
                              {sub.icon && (
                                <sub.icon
                                  className={item.accent ? "text-(--nav-accent) opacity-80" : "text-muted-foreground"}
                                />
                              )}
                              <span>{sub.title}</span>
                              {sub.badge != null && (
                                <span className="ml-auto">{sub.badge}</span>
                              )}
                            </a>
                          </SidebarMenuSubButton>
                        </SidebarMenuSubItem>
                      )
                    })}
                  </SidebarMenuSub>
                ) : null}
              </SidebarMenuItem>
            )
          })}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  )
}
