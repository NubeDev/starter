// Weekday × hour heatmap. Reveals out-of-hours consumption at a
// glance — the highest-signal anomaly view for energy/water at
// portfolio scale.

import * as React from "react";

import { asEpochMs, asNumber } from "../types";
import type { UsageBucketRow } from "../types";
import { DAY_LABELS } from "./presets";
import { Empty } from "./prims";

export function UsageHeatmap({
  rows, selectedHosts, unit,
}: {
  rows: ReadonlyArray<UsageBucketRow>;
  selectedHosts: ReadonlyArray<string>;
  unit: string | null;
}): React.ReactElement {
  const { grid, max } = React.useMemo(() => {
    // grid[day][hour] = { sum, n } → averaged at render time.
    const sums: number[][] = Array.from({ length: 7 }, () => new Array<number>(24).fill(0));
    const counts: number[][] = Array.from({ length: 7 }, () => new Array<number>(24).fill(0));
    const sel = new Set(selectedHosts);
    for (const r of rows) {
      if (!sel.has(r.host_uuid)) continue;
      const v = asNumber(r.avg_value);
      const t = asEpochMs(r.bucket);
      if (v === null || t === null) continue;
      const d = new Date(t);
      // JS: Sun=0..Sat=6 → remap to Mon=0..Sun=6 for ISO display.
      const dow = (d.getDay() + 6) % 7;
      const hr = d.getHours();
      sums[dow]![hr]! += v;
      counts[dow]![hr]! += 1;
    }
    let max = 0;
    const grid: Array<Array<number | null>> = sums.map((row, d) =>
      row.map((s, h) => {
        const n = counts[d]![h]!;
        if (n === 0) return null;
        const avg = s / n;
        if (avg > max) max = avg;
        return avg;
      }),
    );
    return { grid, max };
  }, [rows, selectedHosts]);

  if (max <= 0) return <Empty>Not enough samples yet.</Empty>;

  return (
    <div className="overflow-x-auto">
      <div className="inline-flex flex-col gap-1 min-w-full">
        {/* Hour-of-day header */}
        <div
          className="grid gap-[2px] text-[0.6rem] text-muted-foreground tabular-nums"
          style={{ gridTemplateColumns: "2.5rem repeat(24, minmax(0, 1fr))" }}
        >
          <span />
          {Array.from({ length: 24 }, (_, h) => (
            <span key={h} className="text-center" aria-hidden="true">
              {h % 3 === 0 ? h : ""}
            </span>
          ))}
        </div>
        {grid.map((row, d) => (
          <div
            key={d}
            className="grid gap-[2px] items-center"
            style={{ gridTemplateColumns: "2.5rem repeat(24, minmax(0, 1fr))" }}
          >
            <span className="text-[0.65rem] text-muted-foreground">{DAY_LABELS[d]}</span>
            {row.map((v, h) => {
              const t = v === null ? 0 : v / max; // 0..1
              const bg = v === null
                ? "rgba(148,163,184,0.06)"
                // Tealish gradient on top of the dark glass.
                : `rgba(45,212,191,${(0.12 + t * 0.78).toFixed(3)})`;
              const tip = v === null
                ? `${DAY_LABELS[d]} ${h.toString().padStart(2, "0")}:00 — no data`
                : `${DAY_LABELS[d]} ${h.toString().padStart(2, "0")}:00 — ${v.toFixed(2)}${unit ? ` ${unit}` : ""}`;
              return (
                <div
                  key={h}
                  title={tip}
                  aria-label={tip}
                  className="h-6 rounded-[3px] ring-1 ring-white/5"
                  style={{ background: bg }}
                />
              );
            })}
          </div>
        ))}
        <div className="flex items-center gap-2 mt-1 text-[0.65rem] text-muted-foreground">
          <span>low</span>
          <span
            className="inline-block h-2 w-32 rounded-sm"
            style={{
              background:
                "linear-gradient(90deg, rgba(45,212,191,0.12) 0%, rgba(45,212,191,0.9) 100%)",
            }}
            aria-hidden="true"
          />
          <span>high</span>
          <span className="ml-auto">peak {max.toFixed(2)}{unit ? ` ${unit}` : ""}</span>
        </div>
      </div>
    </div>
  );
}
