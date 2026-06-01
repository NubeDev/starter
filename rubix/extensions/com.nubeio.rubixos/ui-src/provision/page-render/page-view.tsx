// `page-view.tsx` — fetch bc_widgets_by_page, group by device, render cards.
import * as React from "react";
import { widgetsByPage } from "../bc-api";
import type { WidgetRow } from "../bc-types";
import { DeviceCard } from "./device-card";

function groupByDevice(rows: ReadonlyArray<WidgetRow>): Map<string, WidgetRow[]> {
  const m = new Map<string, WidgetRow[]>();
  for (const r of rows) {
    const list = m.get(r.device_id) ?? [];
    list.push(r);
    m.set(r.device_id, list);
  }
  return m;
}

export function PageView({ pageId }: { pageId: string }): React.ReactElement {
  const [rows, setRows] = React.useState<ReadonlyArray<WidgetRow>>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!pageId) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    widgetsByPage(pageId)
      .then((rs) => !cancelled && setRows(rs))
      .catch((e: unknown) =>
        !cancelled && setError(e instanceof Error ? e.message : String(e)),
      )
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [pageId]);

  if (!pageId) {
    return <p className="text-sm italic text-muted-foreground">Select a page to preview.</p>;
  }
  if (error) {
    return (
      <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
        {error}
      </div>
    );
  }
  if (loading) {
    return <p className="text-sm italic text-muted-foreground">loading…</p>;
  }
  if (rows.length === 0) {
    return <p className="text-sm italic text-muted-foreground">No widgets on this page yet.</p>;
  }

  const groups = [...groupByDevice(rows).entries()];
  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
      {groups.map(([deviceId, widgets]) => (
        <DeviceCard key={deviceId} deviceId={deviceId} widgets={widgets} />
      ))}
    </div>
  );
}
