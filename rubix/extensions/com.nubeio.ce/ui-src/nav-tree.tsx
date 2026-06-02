// `nav-tree.tsx` — sidebar nav contribution for `com.nubeio.ce`.
//
// Mirrors the host's NavGroup markup (data-slot / data-sidebar attrs +
// the exact Tailwind classes the host's shadcn Sidebar primitives
// emit) — we can't import those primitives (React is the only shared
// singleton), so we hand-roll the markup. Keep in lockstep with
// `rubix/frontend/src/components/ui/sidebar.tsx`.

import * as React from "react";
import { Cpu, LayoutGrid } from "lucide-react";

import { BlockShell } from "@nube/starter-ext-sdk-ts";
import { EXTENSION_ID } from "./types";

const TREE = [
  { title: "Devices", href: `/extensions/${EXTENSION_ID}/devices`, icon: LayoutGrid },
];

export default function NavTree(): React.ReactElement {
  return (
    <BlockShell>
      <NavTreeInner />
    </BlockShell>
  );
}

const MENU_BUTTON_CLASS =
  "peer/menu-button flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-start text-sm outline-hidden ring-sidebar-ring transition-[width,height,padding] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0 h-8 no-underline";

function NavTreeInner(): React.ReactElement {
  const path = typeof window !== "undefined" ? window.location.pathname : "";
  return (
    <div
      data-slot="sidebar-group"
      data-sidebar="group"
      className="relative flex w-full min-w-0 flex-col p-2"
    >
      <div
        data-slot="sidebar-group-label"
        data-sidebar="group-label"
        className="flex h-8 shrink-0 items-center gap-2 rounded-md px-2 text-xs font-medium text-sidebar-foreground/70"
      >
        <Cpu className="size-4" /> Control Engine
      </div>
      <ul
        data-slot="sidebar-menu"
        data-sidebar="menu"
        className="flex w-full min-w-0 flex-col gap-1"
      >
        {TREE.map((leaf) => {
          const isActive = path === leaf.href || path.startsWith(leaf.href + "/");
          const Icon = leaf.icon;
          return (
            <li
              key={leaf.href}
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
                <Icon />
                <span>{leaf.title}</span>
              </a>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
