// `nav.tsx` — a sidebar *navigation* contribution for `com.acme.devices`.
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
// The component name `DevicesNav` MUST match `contributes.ui.exposes[*].name` in
// `block.yaml`.

import * as React from "react";

import "./app.css";

const EXTENSION_ID = "com.acme.devices";
// Target the host's per-extension page route `/x/:extId` — the same convention
// `com.nexus.demo` uses — which mounts this extension's `main`-slot components
// into the content area. The bare route mounts the *first* `main` component
// (`DevicesDashboard`); the `/dashboard` and `/provision` suffixes select an
// explicit one, matching the `module` paths in `block.yaml`.
const BASE = `/x/${EXTENSION_ID}`;

// Each nav entry is a plain `<a href>` (NOT a router `NavLink`) because the
// federation host does not share `react-router-dom` as a singleton — a remote's
// own `NavLink` wouldn't see the host Router. The host wraps the `sidebar-nav`
// slot in a click interceptor that catches the anchor click and routes it
// through the host router, so a simple link still does SPA navigation.
const ITEMS: { href: string; icon: string; label: string }[] = [
  { href: `${BASE}/app`, icon: "📱", label: "Get started (app)" },
  { href: `${BASE}/dashboard`, icon: "📊", label: "Devices dashboard" },
  { href: `${BASE}/provision`, icon: "🔧", label: "Provision device" },
];

export default function DevicesNav(): React.ReactElement {
  // These entries render in the host's `sidebar-nav` slot — OUTSIDE the panel's
  // `data-ext-id` wrapper — so the wrapper carries its own `data-ext-id` for the
  // scoped Tailwind bundle to match. Sidebar-menu-button shape via host design
  // tokens so they sit flush with the native nav items.
  return (
    <div data-ext-id={EXTENSION_ID} className="flex flex-col gap-0.5">
      {ITEMS.map((it) => (
        <a
          key={it.href}
          href={it.href}
          className="flex h-8 items-center gap-2 rounded-md px-2 text-sm text-sidebar-foreground/80 outline-none ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2"
        >
          <span
            aria-hidden
            className="grid size-5 shrink-0 place-items-center rounded bg-primary/15 text-primary"
          >
            {it.icon}
          </span>
          <span className="truncate">{it.label}</span>
        </a>
      ))}
    </div>
  );
}
