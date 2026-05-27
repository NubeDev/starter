// `ui/sidebar.tsx` — compact Sidebar panel for `com.rubix.example`.
//
// Mounted into `<ExtensionSlot id="sidebar">` on the rubix frontend
// (see rubix/frontend/src/components/layout/app-sidebar.tsx). The
// host's Module Federation runtime loads this module via the
// `./Sidebar` expose declared in vite.config.ts and surfaces it in
// the AppSidebar once an operator presses "Load UI" on the
// /extensions admin page.
//
// Kept intentionally small — the AppSidebar is a navigation surface,
// not a dashboard. We render the extension id, version, a tiny live
// "tools / tables" pill row, and a deep-link to the full `Main` panel
// on /extensions. Heavy charts and tables live in `main.tsx`.

import * as React from "react";

import { BlockShell, useSlotContext } from "@nube/starter-ext-sdk-ts";

import { EXTENSION_ID } from "./types";
import type { ExtensionDetail } from "./types";

export default function Sidebar(): React.ReactElement {
  return (
    <BlockShell>
      <SidebarInner />
    </BlockShell>
  );
}

function SidebarInner(): React.ReactElement {
  const slot = useSlotContext();
  const [detail, setDetail] = React.useState<ExtensionDetail | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
      credentials: "same-origin",
      headers: { accept: "application/json" },
    })
      .then(async (res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return (await res.json()) as ExtensionDetail;
      })
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const c = detail?.manifest?.contributes ?? {};
  const toolCount = (c.tools ?? []).length;
  const tableCount = (c.warehouse_tables ?? []).length;
  const ruleCount = (c.anomaly_rules ?? []).length;
  const version = detail?.manifest?.version ?? null;

  return (
    <section
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      style={{
        margin: "0.25rem 0.5rem",
        padding: "0.5rem 0.625rem",
        borderRadius: "0.5rem",
        border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
        background: "var(--color-surface, transparent)",
        color: "var(--color-foreground, inherit)",
        display: "flex",
        flexDirection: "column",
        gap: "0.35rem",
        fontSize: "0.75rem",
      }}
    >
      <header
        style={{
          display: "flex",
          alignItems: "baseline",
          justifyContent: "space-between",
          gap: "0.5rem",
        }}
      >
        <strong style={{ fontSize: "0.78rem" }}>Rubix Example</strong>
        {version ? (
          <span style={{ opacity: 0.6 }}>v{version}</span>
        ) : null}
      </header>

      {error ? (
        <p role="alert" style={{ margin: 0, opacity: 0.8 }}>
          {error}
        </p>
      ) : (
        <ul
          style={{
            margin: 0,
            padding: 0,
            listStyle: "none",
            display: "flex",
            flexWrap: "wrap",
            gap: "0.25rem",
          }}
        >
          <Pill label="tools" count={toolCount} />
          <Pill label="tables" count={tableCount} />
          <Pill label="rules" count={ruleCount} />
        </ul>
      )}

      <a
        href="/extensions"
        style={{
          alignSelf: "flex-start",
          fontSize: "0.72rem",
          textDecoration: "none",
          opacity: 0.85,
        }}
      >
        open full panel →
      </a>
    </section>
  );
}

function Pill({
  label,
  count,
}: {
  label: string;
  count: number;
}): React.ReactElement {
  return (
    <li
      style={{
        padding: "0.1rem 0.4rem",
        borderRadius: "999px",
        border: "1px solid var(--color-border, rgba(0,0,0,0.12))",
        opacity: count > 0 ? 1 : 0.5,
      }}
    >
      {label}: <strong>{count}</strong>
    </li>
  );
}
