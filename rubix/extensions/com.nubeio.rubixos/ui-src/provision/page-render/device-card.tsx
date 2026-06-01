// `device-card.tsx` — one device's widgets grouped into a titled card.
import * as React from "react";
import { renderWidget } from "../widgets";
import type { WidgetRow } from "../bc-types";

export function DeviceCard({
  deviceId,
  widgets,
}: {
  deviceId: string;
  widgets: ReadonlyArray<WidgetRow>;
}): React.ReactElement {
  const primary = widgets.find((w) => w.role === "primary");
  const title = primary?.title ?? deviceId;
  const points = widgets.filter((w) => w.point_id !== null);
  return (
    <section className="rounded-lg border border-border/60 bg-card text-card-foreground">
      <header className="border-b border-border/60 px-3 py-2">
        <div className="text-sm font-medium">{title}</div>
        <div className="font-mono text-xs text-muted-foreground">{deviceId}</div>
      </header>
      <div className="grid grid-cols-1 gap-4 p-3 sm:grid-cols-2">
        {points.length === 0 ? (
          <p className="text-sm italic text-muted-foreground">No point widgets on this device.</p>
        ) : (
          points.map((w) => (
            <div key={w.widget_id} className="rounded-md border border-border/40 p-3">
              {renderWidget(w.widget, { title: w.title ?? w.widget, widget: w.widget })}
            </div>
          ))
        )}
      </div>
    </section>
  );
}
