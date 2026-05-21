import type { ReactNode } from "react"
import { useLocation } from "react-router-dom"
import {
  IconSitemap,
  IconRobot,
  IconLayoutDashboard,
  IconSparkles,
  IconSettings,
  IconCommand,
} from "@tabler/icons-react"

import { Separator } from "@/components/ui/separator"
import { SidebarTrigger } from "@/components/ui/sidebar"

type SectionMeta = {
  icon: React.ComponentType<{ className?: string }>
  accent: string
}

function sectionFor(pathname: string): SectionMeta {
  if (pathname.startsWith("/agents"))
    return { icon: IconRobot, accent: "var(--accent-agents)" }
  if (pathname.startsWith("/pages"))
    return { icon: IconLayoutDashboard, accent: "var(--accent-info)" }
  if (pathname.startsWith("/skills"))
    return { icon: IconSparkles, accent: "var(--accent-success)" }
  if (pathname.startsWith("/settings"))
    return { icon: IconSettings, accent: "var(--accent-settings)" }
  return { icon: IconSitemap, accent: "var(--accent-flows)" }
}

export function SiteHeader({
  title,
  actions,
}: {
  title: string
  actions?: ReactNode
}) {
  const { pathname } = useLocation()
  const { icon: Icon, accent } = sectionFor(pathname)

  return (
    <header
      className="glass sticky top-0 z-20 flex h-(--header-height) shrink-0 items-center gap-2 border-b border-border/60 transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-(--header-height)"
      style={{
        backgroundImage: `linear-gradient(90deg, color-mix(in oklab, ${accent} 10%, transparent), transparent 40%)`,
      }}
    >
      <div className="flex w-full items-center gap-2 px-4 lg:gap-3 lg:px-6">
        <SidebarTrigger className="-ml-1" />
        <Separator
          orientation="vertical"
          className="mx-1 data-[orientation=vertical]:h-4"
        />

        <span
          aria-hidden
          className="grid size-7 place-items-center rounded-lg border border-border/50 bg-background/60 shadow-xs"
          style={{
            color: accent,
            boxShadow: `0 6px 18px -10px color-mix(in oklab, ${accent} 65%, transparent)`,
          }}
        >
          <Icon className="size-4" />
        </span>
        <h1 className="text-[15px] font-semibold tracking-tight">{title}</h1>

        <div className="ml-auto flex items-center gap-2">
          <button
            type="button"
            className="hidden h-8 items-center gap-2 rounded-md border border-border/60 bg-background/60 px-2.5 text-xs text-muted-foreground shadow-xs transition-colors hover:text-foreground md:flex"
            title="Search (coming soon)"
          >
            <IconCommand className="size-3.5" />
            <span>Search</span>
            <kbd className="rounded border border-border/60 bg-muted/60 px-1 text-[10px] font-medium">
              ⌘K
            </kbd>
          </button>
          {actions}
        </div>
      </div>
    </header>
  )
}
