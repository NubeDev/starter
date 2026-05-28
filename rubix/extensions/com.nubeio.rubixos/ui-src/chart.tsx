// `chart.tsx` — small dependency-free SVG line chart for bucketed
// history data. Uses only the host's CSS tokens (currentColor +
// Tailwind theme classes) so it inherits the rubix theme.

import * as React from "react";
import type { HistoryBucketRow } from "./types";
import { asNumber } from "./types";

export function HistoryLineChart({
  rows,
  height = 220,
}: {
  rows: ReadonlyArray<HistoryBucketRow>;
  height?: number;
}): React.ReactElement | null {
  if (rows.length === 0) return null;

  // Project (bucket, avg_value) pairs into chart space.
  const points = rows
    .map((r) => ({
      t: Date.parse(r.bucket),
      v: asNumber(r.avg_value),
    }))
    .filter((p) => Number.isFinite(p.t) && p.v !== null) as Array<{ t: number; v: number }>;
  if (points.length === 0) return null;

  const tMin = points[0]!.t;
  const tMax = points[points.length - 1]!.t;
  const vMin = Math.min(...points.map((p) => p.v));
  const vMax = Math.max(...points.map((p) => p.v));
  const vSpan = vMax - vMin || 1;
  const tSpan = tMax - tMin || 1;

  const width = 720;
  const padL = 48;
  const padR = 12;
  const padT = 12;
  const padB = 28;
  const innerW = width - padL - padR;
  const innerH = height - padT - padB;

  const xOf = (t: number) => padL + ((t - tMin) / tSpan) * innerW;
  const yOf = (v: number) => padT + innerH - ((v - vMin) / vSpan) * innerH;

  const path = points
    .map((p, i) => `${i === 0 ? "M" : "L"}${xOf(p.t).toFixed(1)},${yOf(p.v).toFixed(1)}`)
    .join(" ");

  // Y-axis ticks (4 evenly spaced, including bounds).
  const yTicks = [0, 1, 2, 3].map((i) => vMin + (vSpan * i) / 3);
  // X-axis ticks: 5 evenly spaced.
  const xTicks = [0, 1, 2, 3, 4].map((i) => tMin + (tSpan * i) / 4);

  return (
    <svg
      width="100%"
      viewBox={`0 0 ${width} ${height}`}
      role="img"
      aria-label="History (time-bucketed average)"
      className="block"
    >
      {/* y gridlines + labels */}
      {yTicks.map((v, i) => {
        const y = yOf(v);
        return (
          <g key={`y${i}`} opacity={0.6}>
            <line
              x1={padL}
              y1={y}
              x2={width - padR}
              y2={y}
              stroke="currentColor"
              strokeWidth={0.5}
              strokeDasharray="2 3"
              opacity={0.4}
            />
            <text
              x={padL - 6}
              y={y + 3}
              fontSize={10}
              textAnchor="end"
              fill="currentColor"
              opacity={0.8}
            >
              {v.toFixed(2)}
            </text>
          </g>
        );
      })}

      {/* x labels */}
      {xTicks.map((t, i) => {
        const x = xOf(t);
        const d = new Date(t);
        const label = `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
        return (
          <text
            key={`x${i}`}
            x={x}
            y={height - 8}
            fontSize={10}
            textAnchor="middle"
            fill="currentColor"
            opacity={0.75}
          >
            {label}
          </text>
        );
      })}

      {/* line */}
      <path d={path} stroke="currentColor" strokeWidth={1.4} fill="none" className="text-primary" />

      {/* dots on each bucket */}
      {points.map((p, i) => (
        <circle
          key={i}
          cx={xOf(p.t)}
          cy={yOf(p.v)}
          r={1.8}
          className="fill-primary"
        />
      ))}
    </svg>
  );
}
