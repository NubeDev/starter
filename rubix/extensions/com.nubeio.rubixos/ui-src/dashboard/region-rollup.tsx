// Region (Australian state) rollup card grid.  With 100+ buildings
// the flat per-site view collapses; this rollup gives a stable
// portfolio-wide read no matter the cardinality.

import * as React from "react";

import { asEpochMs, asNumber } from "../types";
import type { UsageBucketRow } from "../types";
import { Sparkline } from "../sparkline";
import { geoForHost } from "../sites-geo";
import { fmtBig, stateOf } from "./helpers";
import { PillBtn } from "./prims";

export interface RegionBucket {
  state: string;
  hostUuids: ReadonlyArray<string>;
  total: number;
  share: number; // 0..1 of grandTotal (0 if grandTotal == 0)
  selectedCount: number;
  spark: ReadonlyArray<number | null>;
}

export function buildRegions(
  allHosts: ReadonlyArray<{ uuid: string; name: string }>,
  totalsByHost: ReadonlyMap<string, number>,
  bucketRows: ReadonlyArray<UsageBucketRow>,
  selectedHosts: ReadonlyArray<string>,
  grandTotal: number,
): ReadonlyArray<RegionBucket> {
  const byState = new Map<string, string[]>();
  for (const h of allHosts) {
    const s = stateOf(geoForHost(h.uuid)?.locality);
    const list = byState.get(s) ?? [];
    list.push(h.uuid);
    byState.set(s, list);
  }
  // Pre-index bucket rows by host for spark aggregation.
  const byHostBuckets = new Map<string, Array<{ t: number; v: number }>>();
  for (const r of bucketRows) {
    const t = asEpochMs(r.bucket);
    if (t === null) continue;
    const v = asNumber(r.avg_value);
    if (v === null) continue;
    const list = byHostBuckets.get(r.host_uuid) ?? [];
    list.push({ t, v });
    byHostBuckets.set(r.host_uuid, list);
  }
  const sel = new Set(selectedHosts);
  const out: RegionBucket[] = [];
  for (const [state, uuids] of byState) {
    const total = uuids.reduce((s, u) => s + (totalsByHost.get(u) ?? 0), 0);
    // Sum bucket values across this region's hosts per timestamp.
    const sumByT = new Map<number, number>();
    for (const u of uuids) {
      const pts = byHostBuckets.get(u) ?? [];
      for (const p of pts) sumByT.set(p.t, (sumByT.get(p.t) ?? 0) + p.v);
    }
    const ts = Array.from(sumByT.keys()).sort((a, b) => a - b);
    const spark = ts.map((t) => sumByT.get(t) ?? null);
    out.push({
      state,
      hostUuids: uuids,
      total,
      share: grandTotal > 0 ? total / grandTotal : 0,
      selectedCount: uuids.filter((u) => sel.has(u)).length,
      spark,
    });
  }
  // Largest contributor first; "—" pushed to the end.
  out.sort((a, b) => {
    if (a.state === "—" && b.state !== "—") return 1;
    if (b.state === "—" && a.state !== "—") return -1;
    return b.total - a.total;
  });
  return out;
}

export function RegionRollup({
  regions, unit, focusRegion, onFocusRegion, onSelectRegion, onClearRegion,
}: {
  regions: ReadonlyArray<RegionBucket>;
  unit: string | null;
  focusRegion: string | null;
  onFocusRegion: (state: string | null) => void;
  onSelectRegion: (uuids: ReadonlyArray<string>) => void;
  onClearRegion: (uuids: ReadonlyArray<string>) => void;
}): React.ReactElement {
  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3">
      {regions.map((r) => {
        const allOn = r.selectedCount === r.hostUuids.length;
        const noneOn = r.selectedCount === 0;
        const focused = focusRegion === r.state;
        return (
          <div
            key={r.state}
            className={
              "ext-glass p-3 flex flex-col gap-2 transition-shadow " +
              (focused ? "ext-glass--accent ring-1 ring-primary/40" : "")
            }
          >
            <div className="flex items-baseline justify-between gap-2">
              <button
                type="button"
                onClick={() => onFocusRegion(focused ? null : r.state)}
                className={
                  "text-sm font-semibold tracking-tight cursor-pointer " +
                  "hover:text-primary transition-colors focus:outline-none " +
                  "focus-visible:ring-2 focus-visible:ring-primary rounded-sm"
                }
                title={focused ? "Clear focus" : `Focus dashboard on ${r.state}`}
                aria-pressed={focused}
              >
                {r.state}
                {focused ? <span className="ml-1 text-primary" aria-hidden="true">●</span> : null}
              </button>
              <div className="ext-eyebrow tabular-nums">
                {r.selectedCount}/{r.hostUuids.length} sites
              </div>
            </div>
            <div className="ext-num text-xl font-semibold leading-tight">
              {fmtBig(r.total)}
              {unit ? <span className="text-xs text-muted-foreground ml-1">{unit}</span> : null}
            </div>
            <div className="flex items-center gap-2">
              <div className="h-1.5 flex-1 rounded-full bg-muted/40 overflow-hidden">
                <div
                  className="h-full rounded-full bg-primary/70"
                  style={{ width: `${Math.max(2, r.share * 100)}%` }}
                />
              </div>
              <span className="ext-num text-[0.7rem] text-muted-foreground tabular-nums w-10 text-right">
                {(r.share * 100).toFixed(0)}%
              </span>
            </div>
            <div className="h-7 -mx-1 text-primary">
              <Sparkline values={r.spark} width={240} height={28} color="currentColor" />
            </div>
            <div className="flex gap-1 pt-1">
              <PillBtn
                active={allOn}
                onClick={() => onSelectRegion(r.hostUuids)}
              >
                all
              </PillBtn>
              <PillBtn
                active={noneOn}
                onClick={() => onClearRegion(r.hostUuids)}
              >
                none
              </PillBtn>
              <PillBtn
                active={focused}
                onClick={() => onFocusRegion(focused ? null : r.state)}
              >
                {focused ? "focused" : "focus"}
              </PillBtn>
            </div>
          </div>
        );
      })}
    </div>
  );
}
