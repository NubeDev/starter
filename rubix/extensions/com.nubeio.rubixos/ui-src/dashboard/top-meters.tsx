import * as React from "react";

import { EXTENSION_ID } from "../types";
import type { UsagePerMeterRow } from "../types";
import { fmtBig } from "./helpers";
import { Empty } from "./prims";

export function TopMeters({
  rows, unit, hostName, allRows,
}: {
  rows: ReadonlyArray<UsagePerMeterRow>;
  unit: string | null;
  hostName: (uuid: string) => string;
  /** Full per-meter set for the window. Used to compute the
   *  z-score baseline so the "outlier" badge means something at
   *  portfolio scale (one meter relative to its peers). */
  allRows: ReadonlyArray<UsagePerMeterRow>;
}): React.ReactElement {
  // z-score across all per-meter avg values in window. With 1000+
  // meters this stays a stable baseline; with few meters MAD would
  // be more robust but z is good enough as a first-line flag.
  // (Computed before any early return to keep hook order stable.)
  const zByPoint = React.useMemo(() => {
    const vals = allRows
      .map((r) => Number(r.avg_value))
      .filter((v) => Number.isFinite(v) && v > 0);
    if (vals.length < 8) return new Map<string, number>(); // not enough peers
    const mean = vals.reduce((s, v) => s + v, 0) / vals.length;
    const variance = vals.reduce((s, v) => s + (v - mean) ** 2, 0) / vals.length;
    const sd = Math.sqrt(variance);
    const out = new Map<string, number>();
    if (sd <= 0) return out;
    for (const r of allRows) {
      const v = Number(r.avg_value);
      if (Number.isFinite(v)) out.set(r.point_uuid, (v - mean) / sd);
    }
    return out;
  }, [allRows]);

  if (rows.length === 0) return <Empty>No data.</Empty>;
  const max = Math.max(1, ...rows.map((r) => Number(r.avg_value) || 0));

  return (
    <ol className="flex flex-col gap-1.5 m-0 p-0 list-none">
      {rows.map((r, i) => {
        const v = Number(r.avg_value) || 0;
        const pct = (v / max) * 100;
        const z = zByPoint.get(r.point_uuid);
        const outlier = z !== undefined && Math.abs(z) >= 2;
        return (
          <li key={r.point_uuid} className="grid grid-cols-[1.5rem_1fr_5rem] items-center gap-2 text-xs">
            <span className="text-muted-foreground tabular-nums">{i + 1}</span>
            <div className="min-w-0">
              <div className="flex items-center gap-1.5 min-w-0">
                <a
                  href={`/extensions/${EXTENSION_ID}/history?point=${encodeURIComponent(r.point_uuid)}`}
                  className="text-foreground hover:text-primary truncate"
                >
                  {r.name ?? r.point_uuid}
                </a>
                {outlier ? (
                  <span
                    className={
                      "shrink-0 inline-flex items-center gap-0.5 px-1 py-px " +
                      "rounded-md text-[0.6rem] font-semibold tabular-nums " +
                      (z! > 0
                        ? "bg-amber-400/15 text-amber-300 border border-amber-400/40"
                        : "bg-sky-400/15 text-sky-300 border border-sky-400/40")
                    }
                    title={`${z! > 0 ? "Above" : "Below"} portfolio peers (z = ${z!.toFixed(2)})`}
                  >
                    ⚠ z {z! > 0 ? "+" : ""}{z!.toFixed(1)}
                  </span>
                ) : null}
              </div>
              <div className="ext-eyebrow truncate">
                {r.host_uuid ? hostName(r.host_uuid) : "—"} · {r.device_name ?? "—"}
              </div>
              <div className="mt-1 h-1.5 rounded-full bg-muted/40 overflow-hidden">
                <div
                  className={
                    "h-full rounded-full " +
                    (outlier && z! > 0 ? "bg-amber-400/80" : "bg-primary/70")
                  }
                  style={{ width: `${pct}%` }}
                />
              </div>
            </div>
            <div className="ext-num text-right tabular-nums">
              {fmtBig(v)}{unit ? <span className="text-muted-foreground ml-1">{unit}</span> : null}
            </div>
          </li>
        );
      })}
    </ol>
  );
}
