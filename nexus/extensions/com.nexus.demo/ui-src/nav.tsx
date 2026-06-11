// `nav.tsx` — the sidebar nav entries for `com.nexus.demo` (`slot: sidebar-nav`).
//
// Plain `<a href>` links (NOT router `NavLink`s): the federation host does not
// share `react-router-dom`, so a remote's own NavLink wouldn't see the host
// Router. The host wraps the `sidebar-nav` slot in a click interceptor that
// routes internal hrefs through the host router, so these stay portable while
// still doing SPA navigation. They target the host's per-extension page route
// `/x/:extId[/sub]`, which mounts this extension's `Main` (`slot: main`) and
// forwards the sub-path as the slot route.
//
// Component name `DemoNav` MUST match `contributes.ui.exposes[*].name`.

import * as React from "react";

const BASE = "/x/com.nexus.demo";

const LINKS: Array<{ label: string; to: string; icon: string }> = [
  { label: "Overview", to: BASE, icon: "▦" },
  { label: "Readings", to: `${BASE}/readings`, icon: "≣" },
  { label: "About", to: `${BASE}/about`, icon: "ⓘ" },
];

export default function DemoNav(): React.ReactElement {
  return (
    <div className="flex flex-col gap-0.5 px-2 py-1">
      <div className="px-2 pb-1 text-xs font-medium text-sidebar-foreground/50">
        Nexus Demo
      </div>
      {LINKS.map((l) => (
        <a
          key={l.to}
          href={l.to}
          className="flex h-8 items-center gap-2 rounded-md px-2 text-sm text-sidebar-foreground/80 outline-none ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2"
        >
          <span
            aria-hidden
            className="grid size-5 shrink-0 place-items-center rounded bg-primary/15 text-primary"
          >
            {l.icon}
          </span>
          <span className="truncate">{l.label}</span>
        </a>
      ))}
    </div>
  );
}
