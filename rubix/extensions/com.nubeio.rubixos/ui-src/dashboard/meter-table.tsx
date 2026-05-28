import * as React from "react";

import { EXTENSION_ID } from "../types";
import type { MeterRow } from "../types";
import { PillBtn } from "./prims";

export function MeterTable({
  meters,
}: {
  meters: ReadonlyArray<MeterRow>;
}): React.ReactElement {
  // Pagination — with 2k meters in the catalog (and growing as
  // sites are added), rendering everything is a layout-thrash
  // hazard. Show 100 at a time + "load more" so the visible cost
  // stays predictable; users who need the full list can keep
  // clicking, and per-meter drill-downs use the existing link.
  const PAGE = 100;
  const [shown, setShown] = React.useState(PAGE);
  React.useEffect(() => { setShown(PAGE); }, [meters]);
  const slice = meters.slice(0, shown);
  const more = meters.length - slice.length;
  return (
    <div className="flex flex-col">
      <div className="overflow-x-auto max-h-[60vh]">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-card/80 backdrop-blur">
            <tr className="text-left text-xs text-muted-foreground">
              <th className="py-1.5 px-2 font-medium">Site</th>
              <th className="py-1.5 px-2 font-medium">Network</th>
              <th className="py-1.5 px-2 font-medium">Device</th>
              <th className="py-1.5 px-2 font-medium">Meter</th>
              <th className="py-1.5 px-2 font-medium text-right">Unit</th>
            </tr>
          </thead>
          <tbody>
            {slice.map((m) => (
              <tr key={m.uuid} className="border-t border-border/40">
                <td className="py-1.5 px-2">{m.host_name ?? <code className="text-xs">{m.host_uuid}</code>}</td>
                <td className="py-1.5 px-2 text-muted-foreground">{m.network_name ?? "\u2014"}</td>
                <td className="py-1.5 px-2 text-muted-foreground">{m.device_name ?? "\u2014"}</td>
                <td className="py-1.5 px-2">
                  <a
                    className="text-primary hover:underline"
                    href={`/extensions/${EXTENSION_ID}/history?point=${encodeURIComponent(m.uuid)}`}
                  >
                    {m.name ?? m.uuid}
                  </a>
                </td>
                <td className="py-1.5 px-2 text-right tabular-nums">{m.unit ?? "\u2014"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {more > 0 ? (
        <div className="flex items-center justify-between gap-3 mt-2 px-1">
          <span className="ext-eyebrow tabular-nums">
            showing {slice.length.toLocaleString()} of {meters.length.toLocaleString()}
          </span>
          <div className="flex gap-1">
            <PillBtn onClick={() => setShown((n) => n + PAGE)}>
              load {Math.min(more, PAGE)} more
            </PillBtn>
            {more > PAGE ? (
              <PillBtn onClick={() => setShown(meters.length)}>show all</PillBtn>
            ) : null}
          </div>
        </div>
      ) : meters.length > PAGE ? (
        <div className="flex items-center justify-between gap-3 mt-2 px-1">
          <span className="ext-eyebrow tabular-nums">
            showing all {meters.length.toLocaleString()}
          </span>
          <PillBtn onClick={() => setShown(PAGE)}>collapse</PillBtn>
        </div>
      ) : null}
    </div>
  );
}
