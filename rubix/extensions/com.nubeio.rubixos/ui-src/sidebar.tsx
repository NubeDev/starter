// `sidebar.tsx` — compact Sidebar panel for `com.nubeio.rubixos`.

import * as React from "react";

import { BlockShell, useSlotContext } from "@nube/starter-ext-sdk-ts";

import { EXTENSION_ID, type ExtensionDetail, type HistoriesSummaryRow } from "./types";
import { fetchTemplate } from "./api";

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
  const [summary, setSummary] = React.useState<HistoriesSummaryRow | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    Promise.all([
      fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
        credentials: "same-origin",
        headers: { accept: "application/json" },
      }).then(async (r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return (await r.json()) as ExtensionDetail;
      }),
      fetchTemplate<HistoriesSummaryRow>(`${EXTENSION_ID}.histories_summary`, {}).catch(() => []),
    ])
      .then(([d, s]) => {
        if (cancelled) return;
        setDetail(d);
        setSummary((s as HistoriesSummaryRow[])[0] ?? null);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => { cancelled = true; };
  }, []);

  const version = detail?.manifest?.version;
  const samples = summary ? Number(summary.sample_count) : null;
  const points = summary ? Number(summary.point_count) : null;

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
      ) : (
        <div className="text-[0.65rem] text-muted-foreground mt-1 space-y-0.5">
          <div>samples: <span className="tabular-nums text-foreground">{fmtInt(samples)}</span></div>
          <div>points:  <span className="tabular-nums text-foreground">{fmtInt(points)}</span></div>
        </div>
      )}
      <a
        href={`/extensions/${EXTENSION_ID}`}
        className="text-xs text-primary hover:underline mt-2 inline-block"
      >
        open dashboard →
      </a>
    </div>
  );
}

function fmtInt(v: number | null): string {
  return v === null || !Number.isFinite(v) ? "—" : v.toLocaleString();
}
