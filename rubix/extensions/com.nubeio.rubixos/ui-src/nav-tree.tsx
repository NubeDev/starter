// `nav-tree.tsx` — Sidebar nav-tree contribution for `com.nubeio.rubixos`.
// Uses the same data-sidebar attributes and Tailwind classes as the host's
// shadcn SidebarGroup/SidebarMenu components so it renders identically.

import * as React from "react";

import { BlockShell } from "@nube/starter-ext-sdk-ts";
import { EXTENSION_ID } from "./types";

interface NavLeaf   { title: string; href: string }
interface NavBranch { title: string; children: NavLeaf[] }
type NavItem = NavLeaf | NavBranch;
function isBranch(item: NavItem): item is NavBranch { return "children" in item; }

const TREE: NavItem[] = [
  { title: "Overview", href: `/extensions/${EXTENSION_ID}` },
  {
    title: "Topology",
    children: [
      { title: "Hosts",    href: `/extensions/${EXTENSION_ID}/hosts`    },
      { title: "Networks", href: `/extensions/${EXTENSION_ID}/networks` },
      { title: "Devices",  href: `/extensions/${EXTENSION_ID}/devices`  },
    ],
  },
  {
    title: "Data",
    children: [
      { title: "Energy & Water",  href: `/extensions/${EXTENSION_ID}/usage`   },
      { title: "Report (print)",  href: `/extensions/${EXTENSION_ID}/report`  },
      { title: "History (chart)", href: `/extensions/${EXTENSION_ID}/history` },
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
  return (
    // Matches SidebarGroup: relative flex w-full min-w-0 flex-col p-2
    <div data-slot="sidebar-group" data-sidebar="group" className="relative flex w-full min-w-0 flex-col p-2">
      {/* Matches SidebarGroupLabel */}
      <div
        data-slot="sidebar-group-label"
        data-sidebar="group-label"
        className="flex h-8 shrink-0 items-center rounded-md px-2 text-xs font-medium text-sidebar-foreground/70"
      >
        RUBIX-OS
      </div>
      {/* Matches SidebarMenu */}
      <ul data-slot="sidebar-menu" data-sidebar="menu" className="flex w-full min-w-0 flex-col gap-1">
        {TREE.map((item) =>
          isBranch(item)
            ? <Branch key={item.title} branch={item} currentPath={path} />
            : <Leaf key={item.href} leaf={item} currentPath={path} top />
        )}
      </ul>
    </div>
  );
}

function Leaf({ leaf, currentPath, top }: { leaf: NavLeaf; currentPath: string; top?: boolean }): React.ReactElement {
  const isActive = currentPath === leaf.href || currentPath.startsWith(leaf.href + "/");
  if (top) {
    return (
      // Matches SidebarMenuItem
      <li data-slot="sidebar-menu-item" data-sidebar="menu-item" className="group/menu-item relative">
        <a
          href={leaf.href}
          data-slot="sidebar-menu-button"
          data-sidebar="menu-button"
          data-active={isActive}
          className="peer/menu-button flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-start text-sm outline-hidden ring-sidebar-ring transition-[width,height,padding] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground h-8 data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground no-underline"
        >
          <span>{leaf.title}</span>
        </a>
      </li>
    );
  }
  return (
    // Matches SidebarMenuSubItem + SidebarMenuSubButton
    <li data-slot="sidebar-menu-sub-item" data-sidebar="menu-sub-item" className="group/menu-sub-item relative">
      <a
        href={leaf.href}
        data-slot="sidebar-menu-sub-button"
        data-sidebar="menu-sub-button"
        data-size="md"
        data-active={isActive}
        className="flex h-7 min-w-0 -translate-x-px items-center gap-2 overflow-hidden rounded-md px-2 text-sidebar-foreground ring-sidebar-ring outline-hidden hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 text-sm data-[active=true]:bg-sidebar-accent data-[active=true]:text-sidebar-accent-foreground no-underline"
      >
        <span>{leaf.title}</span>
      </a>
    </li>
  );
}

function Branch({ branch, currentPath }: { branch: NavBranch; currentPath: string }): React.ReactElement {
  const defaultOpen = branch.children.some(
    (c) => currentPath === c.href || currentPath.startsWith(c.href + "/")
  );
  const [open, setOpen] = React.useState(defaultOpen || true);
  return (
    <li data-slot="sidebar-menu-item" data-sidebar="menu-item" className="group/menu-item relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        data-slot="sidebar-menu-button"
        data-sidebar="menu-button"
        data-state={open ? "open" : "closed"}
        className="group/collapsible peer/menu-button flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-start text-sm outline-hidden ring-sidebar-ring transition-[width,height,padding] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 h-8 bg-transparent border-0 cursor-pointer"
      >
        <span className="flex-1">{branch.title}</span>
        <svg
          width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
          className={"ms-auto shrink-0 transition-transform duration-200 " + (open ? "rotate-90" : "")}
        >
          <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
      {open ? (
        // Matches SidebarMenuSub
        <ul
          data-slot="sidebar-menu-sub"
          data-sidebar="menu-sub"
          className="mx-3.5 flex min-w-0 translate-x-px flex-col gap-1 border-s border-sidebar-border px-2.5 py-0.5"
        >
          {branch.children.map((leaf) => (
            <Leaf key={leaf.href} leaf={leaf} currentPath={currentPath} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}
