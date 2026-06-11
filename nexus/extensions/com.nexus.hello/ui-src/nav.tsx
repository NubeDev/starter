// `nav.tsx` — a sidebar *navigation* contribution for `com.nexus.hello`.
//
// Where `panel.tsx` proves the data loop, this proves the WS-14 `sidebar-nav`
// slot: an extension can add its own entry to the host's primary navigation.
// It renders a plain `<a href>` (NOT a router `NavLink`) because the federation
// host does not share `react-router-dom` as a singleton — a remote's own
// `NavLink` wouldn't see the host Router. Instead the host wraps the
// `sidebar-nav` slot in a click interceptor (mirroring rubix) that catches the
// anchor click and routes it through the host router, so this stays a simple,
// portable link while still doing SPA navigation.
//
// The component name `HelloNav` MUST match `contributes.ui.exposes[*].name` in
// `block.yaml`.

import * as React from "react";

const HREF = "/extensions";

export default function HelloNav(): React.ReactElement {
  // Sidebar-menu-button shape, styled with the host's design tokens so it sits
  // flush with the native nav items. The host interceptor turns the href into
  // an SPA navigation; no JS handler needed here.
  return (
    <a
      href={HREF}
      className="flex h-8 items-center gap-2 rounded-md px-2 text-sm text-sidebar-foreground/80 outline-none ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2"
    >
      <span
        aria-hidden
        className="grid size-5 shrink-0 place-items-center rounded bg-primary/15 text-primary"
      >
        👋
      </span>
      <span className="truncate">Hello Nav</span>
    </a>
  );
}
