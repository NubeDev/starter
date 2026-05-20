// Sidebar primitive: cookie-persisted open/collapsed (`sidebar_state`),
// auto icon-rail under 1024 px. See SIDEBAR.md §1.
import * as React from "react"
import { HugeiconsIcon } from "@hugeicons/react"
import { ArrowRight01Icon } from "@hugeicons/core-free-icons"
import { cn } from "@/lib/utils"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { ScrollArea } from "@/components/ui/scroll-area"

const COOKIE_NAME = "sidebar_state"
const COOKIE_MAX_AGE = 60 * 60 * 24 * 365
const COLLAPSE_BREAKPOINT_PX = 1024

function readCookie(name: string): string | null {
  if (typeof document === "undefined") return null
  const match = document.cookie
    .split("; ")
    .find((r) => r.startsWith(`${name}=`))
  return match ? decodeURIComponent(match.slice(name.length + 1)) : null
}
function writeCookie(name: string, value: string) {
  if (typeof document === "undefined") return
  document.cookie = `${name}=${encodeURIComponent(value)}; path=/; max-age=${COOKIE_MAX_AGE}; samesite=lax`
}

type SidebarMode = "expanded" | "collapsed"

interface SidebarContextValue {
  mode: SidebarMode
  setMode: (mode: SidebarMode) => void
  toggle: () => void
  forcedCollapsed: boolean
}

const SidebarContext = React.createContext<SidebarContextValue | null>(null)

export function useSidebar(): SidebarContextValue {
  const ctx = React.useContext(SidebarContext)
  if (!ctx) throw new Error("useSidebar must be used inside <SidebarProvider>")
  return ctx
}

export interface SidebarProviderProps {
  children: React.ReactNode
  defaultMode?: SidebarMode
}

export function SidebarProvider({
  children,
  defaultMode,
}: SidebarProviderProps) {
  const [forcedCollapsed, setForcedCollapsed] = React.useState(false)
  const [mode, setModeState] = React.useState<SidebarMode>(() => {
    if (defaultMode) return defaultMode
    return readCookie(COOKIE_NAME) === "collapsed" ? "collapsed" : "expanded"
  })
  React.useEffect(() => {
    if (typeof window === "undefined") return
    const mq = window.matchMedia(`(max-width: ${COLLAPSE_BREAKPOINT_PX}px)`)
    const update = () => setForcedCollapsed(mq.matches)
    update()
    mq.addEventListener("change", update)
    return () => mq.removeEventListener("change", update)
  }, [])
  const setMode = React.useCallback((next: SidebarMode) => {
    setModeState(next)
    writeCookie(COOKIE_NAME, next)
  }, [])
  const toggle = React.useCallback(() => {
    setModeState((prev) => {
      const next: SidebarMode = prev === "expanded" ? "collapsed" : "expanded"
      writeCookie(COOKIE_NAME, next)
      return next
    })
  }, [])
  const effective: SidebarMode = forcedCollapsed ? "collapsed" : mode
  const value = React.useMemo<SidebarContextValue>(
    () => ({ mode: effective, setMode, toggle, forcedCollapsed }),
    [effective, setMode, toggle, forcedCollapsed],
  )
  return (
    <SidebarContext.Provider value={value}>{children}</SidebarContext.Provider>
  )
}

export function Sidebar({
  className,
  children,
  ...props
}: React.ComponentProps<"aside">) {
  const { mode } = useSidebar()
  return (
    <aside
      data-slot="sidebar"
      data-state={mode}
      className={cn(
        "group/sidebar relative flex h-full shrink-0 flex-col border-r border-border/60 bg-sidebar/40 transition-[width] duration-200 ease-out",
        mode === "expanded" ? "w-60" : "w-14",
        className,
      )}
      {...props}
    >
      {children}
    </aside>
  )
}

export function SidebarHeader({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-header"
      className={cn(
        "flex h-12 items-center gap-2 border-b border-border/60 px-3 text-sm font-medium",
        className,
      )}
      {...props}
    />
  )
}

export function SidebarContent({
  className,
  children,
}: {
  className?: string
  children?: React.ReactNode
}) {
  return (
    <ScrollArea
      data-slot="sidebar-content"
      className={cn("flex-1", className)}
    >
      <div className="flex flex-col gap-1 p-2">{children}</div>
    </ScrollArea>
  )
}

export interface SidebarGroupProps {
  open?: boolean
  defaultOpen?: boolean
  onOpenChange?: (open: boolean) => void
  children: React.ReactNode
  className?: string
}

export function SidebarGroup({
  open,
  defaultOpen = true,
  onOpenChange,
  children,
  className,
}: SidebarGroupProps) {
  return (
    <Collapsible
      data-slot="sidebar-group"
      open={open}
      defaultOpen={defaultOpen}
      onOpenChange={onOpenChange}
      className={cn("flex flex-col", className)}
    >
      {children}
    </Collapsible>
  )
}

export function SidebarGroupLabel({
  children,
  icon,
  className,
}: {
  children: React.ReactNode
  icon?: React.ReactNode
  className?: string
}) {
  const { mode } = useSidebar()
  return (
    <CollapsibleTrigger
      data-slot="sidebar-group-label"
      className={cn(
        "group/group-label flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground transition-colors hover:bg-accent/40 hover:text-foreground",
        className,
      )}
    >
      <HugeiconsIcon
        icon={ArrowRight01Icon}
        strokeWidth={2}
        className="size-3 shrink-0 transition-transform duration-150 group-data-[state=open]/group-label:rotate-90"
      />
      {icon ? <span className="size-4 shrink-0">{icon}</span> : null}
      {mode === "expanded" ? (
        <span className="truncate text-left">{children}</span>
      ) : null}
    </CollapsibleTrigger>
  )
}

export function SidebarGroupContent({
  children,
  className,
}: {
  children: React.ReactNode
  className?: string
}) {
  return (
    <CollapsibleContent
      data-slot="sidebar-group-content"
      className={cn(
        "overflow-hidden data-closed:animate-collapsible-up data-open:animate-collapsible-down",
        className,
      )}
    >
      <div className="mt-1 flex flex-col gap-0.5 pl-3">{children}</div>
    </CollapsibleContent>
  )
}

function rowClassName(active: boolean | undefined, mode: SidebarMode) {
  return cn(
    "group/sidebar-item flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-sm transition-colors duration-150",
    active
      ? "bg-accent/60 text-accent-foreground"
      : "text-muted-foreground hover:bg-accent/40 hover:text-foreground",
    mode === "collapsed" && "justify-center",
  )
}

export interface SidebarItemProps
  extends Omit<React.ComponentProps<"button">, "children"> {
  active?: boolean
  icon?: React.ReactNode
  asChild?: boolean
  children: React.ReactNode
}

export function SidebarItem({
  active,
  icon,
  className,
  children,
  asChild,
  ...props
}: SidebarItemProps) {
  const { mode } = useSidebar()
  const cls = cn(rowClassName(active, mode), className)
  if (asChild && React.isValidElement(children)) {
    const child = children as React.ReactElement<{ className?: string }>
    return React.cloneElement(child, {
      className: cn(cls, child.props.className),
      "data-slot": "sidebar-item",
      "data-active": active ? "true" : undefined,
      ...props,
    } as React.HTMLAttributes<HTMLElement>)
  }
  return (
    <button
      type="button"
      data-slot="sidebar-item"
      data-active={active ? "true" : undefined}
      className={cls}
      {...props}
    >
      {icon ? <span className="size-4 shrink-0">{icon}</span> : null}
      {mode === "expanded" ? (
        <span className="truncate text-left">{children}</span>
      ) : null}
    </button>
  )
}

export interface SidebarTreeNode {
  id: string
  label: React.ReactNode
  icon?: React.ReactNode
  active?: boolean
  onSelect?: () => void
  render?: (inner: React.ReactNode) => React.ReactNode
  children?: SidebarTreeNode[]
}

export interface SidebarTreeProps {
  nodes: SidebarTreeNode[]
  expanded?: Set<string>
  onExpandedChange?: (next: Set<string>) => void
  defaultExpanded?: string[]
  className?: string
}

export function SidebarTree({
  nodes,
  expanded,
  onExpandedChange,
  defaultExpanded,
  className,
}: SidebarTreeProps) {
  const [internal, setInternal] = React.useState<Set<string>>(
    () => new Set(defaultExpanded ?? []),
  )
  const current = expanded ?? internal
  const toggle = React.useCallback(
    (id: string) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      if (onExpandedChange) onExpandedChange(next)
      if (!expanded) setInternal(next)
    },
    [current, expanded, onExpandedChange],
  )
  return (
    <div data-slot="sidebar-tree" className={cn("flex flex-col", className)}>
      {nodes.map((n) => (
        <SidebarTreeRow
          key={n.id}
          node={n}
          depth={0}
          expanded={current}
          onToggle={toggle}
        />
      ))}
    </div>
  )
}

function SidebarTreeRow({
  node,
  depth,
  expanded,
  onToggle,
}: {
  node: SidebarTreeNode
  depth: number
  expanded: Set<string>
  onToggle: (id: string) => void
}) {
  const { mode } = useSidebar()
  const hasChildren = !!node.children && node.children.length > 0
  const isOpen = expanded.has(node.id)
  const paddingLeft = mode === "expanded" ? 8 + depth * 12 : 0
  const row = (
    <div
      data-slot="sidebar-tree-row"
      data-active={node.active ? "true" : undefined}
      className={cn(rowClassName(node.active, mode), "px-0 pr-2")}
      style={{ paddingLeft: paddingLeft || undefined }}
      onClick={(e) => {
        if (hasChildren) {
          e.preventDefault()
          onToggle(node.id)
        } else {
          node.onSelect?.()
        }
      }}
    >
      {hasChildren ? (
        <HugeiconsIcon
          icon={ArrowRight01Icon}
          strokeWidth={2}
          className={cn(
            "size-3 shrink-0 transition-transform duration-150",
            isOpen && "rotate-90",
          )}
        />
      ) : (
        <span
          aria-hidden
          className={cn("size-3 shrink-0", mode === "collapsed" && "hidden")}
        />
      )}
      {node.icon ? <span className="size-4 shrink-0">{node.icon}</span> : null}
      {mode === "expanded" ? (
        <span className="truncate text-left">{node.label}</span>
      ) : null}
    </div>
  )
  return (
    <>
      {node.render && !hasChildren ? node.render(row) : row}
      {hasChildren && isOpen && mode === "expanded" ? (
        <div className="flex flex-col">
          {node.children!.map((c) => (
            <SidebarTreeRow
              key={c.id}
              node={c}
              depth={depth + 1}
              expanded={expanded}
              onToggle={onToggle}
            />
          ))}
        </div>
      ) : null}
    </>
  )
}
