// `ui/nav-tree.tsx` — Sidebar nav-tree contribution for `com.rubix.example`.
//
// Mounts into `<ExtensionSlot id="sidebar-nav">` in
// rubix/frontend/src/components/layout/app-sidebar.tsx. Renders the
//   Extension Name
//     <tab>
//       <nested tab>
// hierarchy alongside the host's static + live NavGroups.
//
// Built with plain JSX + CSS variables (var(--color-*)) instead of the
// host's shadcn `Sidebar*` primitives — those are project-aliased
// (`@/components/ui/sidebar`) and not safely importable from an
// extension bundle. Reading the host's CSS vars keeps the visual
// language consistent without coupling to the host's component library.

import * as React from "react";

import { BlockShell } from "@nube/starter-ext-sdk-ts";

interface NavLeaf {
  title: string;
  href: string;
}

interface NavBranch {
  title: string;
  children: NavLeaf[];
}

// The tree this extension contributes. Static today; a future
// iteration can pull this from `/api/v1/extensions/<id>` or the SSE
// dashboards feed.
// Per-extension routes resolve at `/extensions/<id>/<route>` via the
// host's catch-all route file (extensions.$extId.$.tsx). The extension
// reads the `<route>` tail with `useExtensionRoute()` and renders the
// matching sub-view from `main.tsx`.
import { EXTENSION_ID } from "./types";

const TREE: NavBranch[] = [
  {
    title: "Customers",
    children: [
      { title: "By country", href: `/extensions/${EXTENSION_ID}/customers/by-country` },
      { title: "Quality issues", href: `/extensions/${EXTENSION_ID}/customers/quality` },
    ],
  },
  {
    title: "Products",
    children: [
      { title: "Low stock", href: `/extensions/${EXTENSION_ID}/products/low-stock` },
      { title: "Catalog", href: `/extensions/${EXTENSION_ID}/products/catalog` },
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
  return (
    <nav
      aria-label="Rubix Example"
      style={{
        margin: "0.25rem 0.5rem",
        fontSize: "0.8125rem",
        color: "var(--color-foreground, inherit)",
      }}
    >
      <div
        style={{
          padding: "0.25rem 0.5rem",
          fontSize: "0.7rem",
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.04em",
          opacity: 0.6,
        }}
      >
        Rubix Example
      </div>
      <ul style={{ margin: 0, padding: 0, listStyle: "none" }}>
        {TREE.map((branch) => (
          <Branch key={branch.title} branch={branch} />
        ))}
      </ul>
    </nav>
  );
}

function Branch({ branch }: { branch: NavBranch }): React.ReactElement {
  const [open, setOpen] = React.useState(true);
  return (
    <li>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        style={{
          width: "100%",
          display: "flex",
          alignItems: "center",
          gap: "0.4rem",
          padding: "0.3rem 0.5rem",
          background: "transparent",
          border: 0,
          color: "inherit",
          font: "inherit",
          cursor: "pointer",
          borderRadius: "0.375rem",
          textAlign: "left",
        }}
      >
        <Chevron open={open} />
        <span>{branch.title}</span>
      </button>
      {open ? (
        <ul
          style={{
            margin: 0,
            paddingInlineStart: "1.25rem",
            listStyle: "none",
            borderInlineStart: "1px solid var(--color-border, rgba(0,0,0,0.12))",
            marginInlineStart: "1rem",
          }}
        >
          {branch.children.map((leaf) => (
            <li key={leaf.href}>
              <a
                href={leaf.href}
                style={{
                  display: "block",
                  padding: "0.25rem 0.5rem",
                  textDecoration: "none",
                  color: "inherit",
                  borderRadius: "0.375rem",
                  opacity: 0.85,
                }}
              >
                {leaf.title}
              </a>
            </li>
          ))}
        </ul>
      ) : null}
    </li>
  );
}

function Chevron({ open }: { open: boolean }): React.ReactElement {
  return (
    <svg
      width="10"
      height="10"
      viewBox="0 0 10 10"
      aria-hidden="true"
      style={{
        transition: "transform 120ms",
        transform: open ? "rotate(90deg)" : "rotate(0deg)",
        flexShrink: 0,
        opacity: 0.7,
      }}
    >
      <path d="M3 1.5 L7 5 L3 8.5" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
