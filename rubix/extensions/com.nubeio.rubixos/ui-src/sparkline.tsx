// `sparkline.tsx` — tiny inline SVG sparkline for site tiles.
//
// No axes, no labels, just the trend curve + a soft area fill. Used
// inside `SiteTile` (Energy & Water dashboard) where we have ~24
// points to show and need it to render in <50px height. SVG is
// fine at this size; bringing up a uPlot canvas per tile would be
// wasteful.

import * as React from "react";

export function Sparkline({
  values,
  width = 120,
  height = 32,
  color = "currentColor",
}: {
  values: ReadonlyArray<number | null>;
  width?: number;
  height?: number;
  color?: string;
}): React.ReactElement | null {
  const valid = values.filter((v): v is number => v !== null && Number.isFinite(v));
  if (valid.length < 2) return null;

  const min = Math.min(...valid);
  const max = Math.max(...valid);
  const span = max - min || 1;
  const stepX = width / (values.length - 1);

  const ptOf = (v: number | null, i: number): [number, number] | null => {
    if (v === null || !Number.isFinite(v)) return null;
    const x = i * stepX;
    const y = height - ((v - min) / span) * (height - 2) - 1;
    return [x, y];
  };

  let path = "";
  let area = "";
  let started = false;
  for (let i = 0; i < values.length; i++) {
    const pt = ptOf(values[i] ?? null, i);
    if (!pt) continue;
    if (!started) {
      path += `M${pt[0].toFixed(1)},${pt[1].toFixed(1)}`;
      area += `M${pt[0].toFixed(1)},${height} L${pt[0].toFixed(1)},${pt[1].toFixed(1)}`;
      started = true;
    } else {
      path += ` L${pt[0].toFixed(1)},${pt[1].toFixed(1)}`;
      area += ` L${pt[0].toFixed(1)},${pt[1].toFixed(1)}`;
    }
  }
  // Close area back down to baseline.
  const lastIdx = values.length - 1;
  area += ` L${(lastIdx * stepX).toFixed(1)},${height} Z`;

  const gradId = React.useId();
  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      width="100%"
      height={height}
      preserveAspectRatio="none"
      role="img"
      aria-label="trend"
      className="block"
    >
      <defs>
        <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity={0.45} />
          <stop offset="100%" stopColor={color} stopOpacity={0} />
        </linearGradient>
      </defs>
      <path d={area} fill={`url(#${gradId})`} />
      <path d={path} fill="none" stroke={color} strokeWidth={1.5} strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}
