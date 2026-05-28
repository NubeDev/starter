import * as React from "react";
import type { AlignedData } from "uplot";

import { asEpochMs, asNumber } from "../types";
import type { UsageBucketRow } from "../types";
import { UplotChart } from "../uplot-chart";
import { PALETTE } from "./presets";

export function UsageTimeSeries({
  rows, hostUuids, hostName, unit, totalsByHost, topN, showAll,
}: {
  rows: ReadonlyArray<UsageBucketRow>;
  hostUuids: ReadonlyArray<string>;
  hostName: (uuid: string) => string;
  unit: string | null;
  totalsByHost: ReadonlyMap<string, number>;
  topN: number;
  showAll: boolean;
}): React.ReactElement {
  const { data, hosts, totalLabel } = React.useMemo(() => {
    const xsSet = new Set<number>();
    for (const r of rows) {
      const t = asEpochMs(r.bucket);
      if (t !== null) xsSet.add(Math.floor(t / 1000));
    }
    const xs = Array.from(xsSet).sort((a, b) => a - b);
    const xIdx = new Map(xs.map((t, i) => [t, i] as const));

    const present = new Set(rows.map((r) => r.host_uuid));
    const selectedPresent = hostUuids.filter((u) => present.has(u));

    // Cap line count to keep the legend usable at 100+ sites:
    // sort selected hosts by total desc, take top-N unless caller
    // explicitly opted in to "show all".
    const ranked = selectedPresent
      .slice()
      .sort((a, b) => (totalsByHost.get(b) ?? 0) - (totalsByHost.get(a) ?? 0));
    const hosts = showAll ? ranked : ranked.slice(0, topN);

    // Per-host series.
    const hostIdx = new Map(hosts.map((u, i) => [u, i] as const));
    const series: Array<Array<number | null>> = hosts.map(() =>
      new Array<number | null>(xs.length).fill(null),
    );
    // "Total" series — sum across ALL selected present hosts per
    // bucket. Independent of the top-N cap so the headline figure
    // is correct.
    const total: Array<number | null> = new Array<number | null>(xs.length).fill(null);
    const selSet = new Set(selectedPresent);

    for (const r of rows) {
      const t = asEpochMs(r.bucket);
      if (t === null) continue;
      const xi = xIdx.get(Math.floor(t / 1000));
      if (xi === undefined) continue;
      const v = asNumber(r.avg_value);
      if (v === null) continue;
      if (selSet.has(r.host_uuid)) {
        total[xi] = (total[xi] ?? 0) + v;
      }
      const hi = hostIdx.get(r.host_uuid);
      if (hi !== undefined) series[hi]![xi] = v;
    }

    const totalLabel = `Σ Total (${selectedPresent.length})`;
    return {
      data: [xs, total, ...series] as AlignedData,
      hosts,
      totalLabel,
    };
  }, [rows, hostUuids, totalsByHost, topN, showAll]);

  const opts = React.useMemo(
    () => ({
      height: 320,
      legend: { show: true, live: true },
      cursor: { drag: { x: true, y: false } },
      scales: { x: { time: true } },
      axes: [
        { stroke: "rgba(148,163,184,0.85)", grid: { stroke: "rgba(148,163,184,0.12)" } },
        {
          stroke: "rgba(148,163,184,0.85)",
          grid: { stroke: "rgba(148,163,184,0.12)" },
          label: unit ?? "",
        },
      ],
      series: [
        { label: "Time" },
        {
          label: totalLabel,
          stroke: "rgba(248,250,252,0.95)",
          width: 2.4,
          spanGaps: true,
        },
        ...hosts.map((u, i) => ({
          label: hostName(u),
          stroke: PALETTE[i % PALETTE.length]!,
          width: 1.4,
          spanGaps: false,
        })),
      ],
    }),
    [hosts, hostName, unit, totalLabel],
  );

  return (
    <UplotChart
      data={data}
      opts={opts}
      schemaKey={"total::" + hosts.join("|") + "::" + (unit ?? "")}
      className="w-full"
    />
  );
}
