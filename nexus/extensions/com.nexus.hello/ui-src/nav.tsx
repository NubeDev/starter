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

import "./app.css";

const EXTENSION_ID = "com.nexus.hello";
// Target the host's per-extension page route `/x/:extId` — the same convention
// `com.nexus.demo` uses — which mounts this extension's `HelloPanel` (`slot:
// main`) into the content area. (Previously this pointed at `/extensions`, the
// admin list.)
const HREF = `/x/${EXTENSION_ID}`;

export default function HelloNav(): React.ReactElement {
  // This entry renders in the host's `sidebar-nav` slot — OUTSIDE the panel's
  // `data-ext-id` wrapper — so it carries its own `data-ext-id` for the scoped
  // Tailwind bundle to match (the scope covers both the element itself and its
  // descendants). The host interceptor turns the href into SPA navigation; no
  // JS handler needed here. Sidebar-menu-button shape via host design tokens so
  // it sits flush with the native nav items.
  return (
    <a
      data-ext-id={EXTENSION_ID}
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
