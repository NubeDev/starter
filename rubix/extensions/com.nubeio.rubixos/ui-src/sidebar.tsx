// `sidebar.tsx` — compact Sidebar panel for `com.nubeio.rubixos`.

import * as React from "react";

import { BlockShell, useSlotContext } from "@nube/starter-ext-sdk-ts";

import { EXTENSION_ID, type ExtensionDetail } from "./types";
import { fetchExtensionDetail } from "./detail";

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

  // The sidebar mounts on every authed route (the host's AppSidebar slot
  // is global). Do not fire warehouse_query here — `histories_summary`
  // is an expensive aggregation against the histories table and would
  // run on `/`, `/devices`, `/flows`, etc., where it has no business
  // being. The dashboard page itself shows live counts; the sidebar
  // only surfaces version + a deep-link.
  React.useEffect(() => {
    let cancelled = false;
    fetchExtensionDetail()
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => { cancelled = true; };
  }, []);

  const version = detail?.manifest?.version;

  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      className="mx-2 my-1 rounded-md border border-border/60 bg-card text-card-foreground px-3 py-2"
    >
      <div className="flex items-baseline justify-between gap-2">
        <div className="text-xs font-semibold">Rubix-OS</div>
        {version ? <span className="text-muted-foreground text-[0.65rem]">v{version}</span> : null}
      </div>
      {error ? (
        <p role="alert" className="text-sm text-destructive mt-1">{error}</p>
      ) : null}
      <a
        href={`/extensions/${EXTENSION_ID}`}
        className="text-xs text-primary hover:underline mt-2 inline-block"
      >
        open dashboard →
      </a>
    </div>
  );
}
