// `nav-tree.tsx` — Sidebar nav-tree contribution for `com.nubeio.rubixos`.
//
// Renders identically to the host's NavGroup (rubix/frontend/src/components/
// layout/nav-group.tsx) by mirroring the exact data-slot/data-sidebar
// attributes, Tailwind class strings, and lucide-react icons that the
// host's shadcn `SidebarMenu*` primitives emit. We can't import those
// primitives directly — the extension is a module-federation remote and
// React is the only externalised singleton — so we hand-roll the markup
// with the same classes the host's `sidebarMenuButtonVariants` cva emits.
//
// When the host's sidebar primitives change classes, this file must be
// updated in lockstep (see `rubix/frontend/src/components/ui/sidebar.tsx`).

import * as React from "react";
import {
  ChevronRight,
  Database,
  FileText,
  LineChart,
  LayoutGrid,
  Network,
  Router,
  Server,
  Zap,
} from "lucide-react";

import { BlockShell } from "@nube/starter-ext-sdk-ts";
import { EXTENSION_ID } from "./types";

type IconCmp = React.ComponentType<{ className?: string }>;

interface NavLeaf {
  title: string;
  href: string;
  icon?: IconCmp;
}
interface NavBranch {
  title: string;
  icon?: IconCmp;
  children: NavLeaf[];
}
type NavItem = NavLeaf | NavBranch;
function isBranch(item: NavItem): item is NavBranch {
  return "children" in item;
}

const TREE: NavItem[] = [
  { title: "Overview", href: `/extensions/${EXTENSION_ID}`, icon: LayoutGrid },
  {
    title: "Topology",
    icon: Network,
    children: [
      { title: "Hosts",    href: `/extensions/${EXTENSION_ID}/hosts`,    icon: Server },
      { title: "Networks", href: `/extensions/${EXTENSION_ID}/networks`, icon: Router },
      { title: "Devices",  href: `/extensions/${EXTENSION_ID}/devices`,  icon: Database },
    ],
  },
  {
    title: "Data",
    icon: Database,
    children: [
      { title: "Energy & Water",  href: `/extensions/${EXTENSION_ID}/usage`,   icon: Zap },
      { title: "Report (print)",  href: `/extensions/${EXTENSION_ID}/report`,  icon: FileText },
      { title: "History (chart)", href: `/extensions/${EXTENSION_ID}/history`, icon: LineChart },
    ],
  },
];

export default function NavTree(): React.ReactElement {
  return (
    <BlockShell>
      <NavTreeInner />
    </BlockShell>
  );
}

function NavTreeInner(): React.ReactElement {
  const path = typeof window !== "undefined" ? window.location.pathname : "";
  // SidebarGroup
  return (
    <div
      data-slot="sidebar-group"
      data-sidebar="group"
      className="relative flex w-full min-w-0 flex-col p-2"
    >
      {/* SidebarGroupLabel */}
      <div
        data-slot="sidebar-group-label"
        data-sidebar="group-label"
        className="flex h-8 shrink-0 items-center rounded-md px-2 text-xs font-medium text-sidebar-foreground/70 ring-sidebar-ring outline-hidden transition-[margin,opacity] duration-200 ease-linear focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0 group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0"
      >
        Rubix-OS
      </div>
      {/* SidebarMenu */}
      <ul
        data-slot="sidebar-menu"
        data-sidebar="menu"
        className="flex w-full min-w-0 flex-col gap-1"
      >
        {TREE.map((item) =>
          isBranch(item) ? (
            <Branch key={item.title} branch={item} currentPath={path} />
          ) : (
            <Leaf key={item.href} leaf={item} currentPath={path} top />
          ),
        )}
      </ul>
    </div>
  );
}

// Matches SidebarMenuButton (top) / SidebarMenuSubButton (nested).
const MENU_BUTTON_CLASS =
  "peer/menu-button flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-start text-sm outline-hidden ring-sidebar-ring transition-[width,height,padding] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 group-has-data-[sidebar=menu-action]/menu-item:pe-8 aria-disabled:pointer-events-none aria-disabled:opacity-50 data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground data-[state=open]:hover:bg-sidebar-accent data-[state=open]:hover:text-sidebar-accent-foreground group-data-[collapsible=icon]:size-8! group-data-[collapsible=icon]:p-2! [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0 h-8 no-underline";

const SUB_BUTTON_CLASS =
  "flex h-7 min-w-0 -translate-x-px items-center gap-2 overflow-hidden rounded-md px-2 text-sidebar-foreground ring-sidebar-ring outline-hidden hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0 [&>svg]:text-inherit text-sm data-[active=true]:bg-sidebar-accent data-[active=true]:text-sidebar-accent-foreground group-data-[collapsible=icon]:hidden no-underline";

function Leaf({
  leaf,
  currentPath,
  top,
}: {
  leaf: NavLeaf;
  currentPath: string;
  top?: boolean;
}): React.ReactElement {
  const isActive =
    currentPath === leaf.href || currentPath.startsWith(leaf.href + "/");
  const Icon = leaf.icon;
  if (top) {
    return (
      // SidebarMenuItem
      <li
        data-slot="sidebar-menu-item"
        data-sidebar="menu-item"
        className="group/menu-item relative"
      >
        <a
          href={leaf.href}
          data-slot="sidebar-menu-button"
          data-sidebar="menu-button"
          data-active={isActive}
          className={MENU_BUTTON_CLASS}
        >
          {Icon ? <Icon /> : null}
          <span>{leaf.title}</span>
        </a>
      </li>
    );
  }
  return (
    // SidebarMenuSubItem
    <li
      data-slot="sidebar-menu-sub-item"
      data-sidebar="menu-sub-item"
      className="group/menu-sub-item relative"
    >
      <a
        href={leaf.href}
        data-slot="sidebar-menu-sub-button"
        data-sidebar="menu-sub-button"
        data-size="md"
        data-active={isActive}
        className={SUB_BUTTON_CLASS}
      >
        {Icon ? <Icon /> : null}
        <span>{leaf.title}</span>
      </a>
    </li>
  );
}

function Branch({
  branch,
  currentPath,
}: {
  branch: NavBranch;
  currentPath: string;
}): React.ReactElement {
  const defaultOpen = branch.children.some(
    (c) => currentPath === c.href || currentPath.startsWith(c.href + "/"),
  );
  const [open, setOpen] = React.useState(defaultOpen);
  const Icon = branch.icon;
  return (
    // SidebarMenuItem wrapping a Collapsible
    <li
      data-slot="sidebar-menu-item"
      data-sidebar="menu-item"
      className="group/collapsible group/menu-item relative"
      data-state={open ? "open" : "closed"}
    >
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        data-slot="sidebar-menu-button"
        data-sidebar="menu-button"
        data-state={open ? "open" : "closed"}
        className={MENU_BUTTON_CLASS + " border-0 bg-transparent cursor-pointer"}
      >
        {Icon ? <Icon /> : null}
        <span>{branch.title}</span>
        <ChevronRight className="ms-auto transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90" />
      </button>
      {open ? (
        // SidebarMenuSub
        <ul
          data-slot="sidebar-menu-sub"
          data-sidebar="menu-sub"
          className="mx-3.5 flex min-w-0 translate-x-px flex-col gap-1 border-s border-sidebar-border px-2.5 py-0.5 group-data-[collapsible=icon]:hidden"
        >
          {branch.children.map((leaf) => (
            <Leaf key={leaf.href} leaf={leaf} currentPath={currentPath} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}
