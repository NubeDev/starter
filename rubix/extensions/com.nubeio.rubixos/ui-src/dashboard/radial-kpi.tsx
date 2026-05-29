// `RadialKpi` — hero metric card.
//
// Adapted from the shadcn "Radial Chart - Text" example
// (https://ui.shadcn.com/charts/radial). Three-zone vertical
// layout (header → chart → footer) so each piece gets a defined
// region and nothing fights for the same pixels. The big number
// lives *inside* the SVG via `<Label content>` on a
// `<PolarRadiusAxis>`, so it stays geometrically centred with
// the bar regardless of card width.

import * as React from "react";
import {
  Label,
  PolarGrid,
  PolarRadiusAxis,
  RadialBar,
  RadialBarChart,
} from "recharts";

import type { RoleAccent } from "./icons";
import { IconTrend } from "./icons";

export function RadialKpi({
  value, unit, deltaPct, deltaLabel, accent, periodLabel,
}: {
  value: string;
  unit?: string | null;
  deltaPct: number | null;
  deltaLabel?: string;
  accent: RoleAccent;
  periodLabel?: string;
}): React.ReactElement {
  // Bar geometry: 0..100 angular domain, fill = clamped |Δ%|.
  // Direction (sign of delta) is encoded in colour, not in length.
  const magnitude = deltaPct === null ? 0 : Math.min(100, Math.abs(deltaPct));
  const goodNews = deltaPct !== null && deltaPct < 0;
  const neutral = deltaPct === null || Math.abs(deltaPct) < 0.5;
  const fillColor = neutral
    ? "#64748b"
    : goodNews
      ? "#10b981"            // emerald — consumption fell
      : accent.cssColor;     // role tint — consumption rose

  const data = [{ name: "delta", value: magnitude, fill: fillColor }];

  // Length-aware sizing for the SVG headline number. Recharts'
  // `Label` takes a render-callback so we hand-place tspan rows;
  // a fixed font-size would crop "164,178.9B" on narrow cards.
  const valueFontSize = pickFontSize(value.length);
  const Icon = accent.Icon;
  const arrow = deltaPct === null ? null
              : deltaPct > 0      ? "↑"
              : deltaPct < 0      ? "↓"
              :                     "→";

  return (
    <div className={"ext-glass relative flex flex-col gap-3 p-4 " + accent.bgTint}>
      {/* ── Header ───────────────────────────────────── */}
      <header className={"flex items-center gap-2 " + accent.text}>
        <Icon size={18} strokeWidth={2.25} />
        <div className="min-w-0">
          <div className="ext-eyebrow !text-current opacity-90 font-semibold">
            {accent.label} total
          </div>
          {periodLabel ? (
            <div className="text-[0.7rem] text-muted-foreground tracking-wide">
              {periodLabel} window
            </div>
          ) : null}
        </div>
      </header>

      {/* ── Chart ────────────────────────────────────── */}
      {/* `aspect-square` + `max-h-[220px]` matches the shadcn
          recipe; chart hugs its container so the centre label
          stays geometrically locked to the arc. */}
      <div className="mx-auto w-full max-w-[260px] aspect-square">
        <RadialBarChart
          width={260}
          height={260}
          data={data}
          startAngle={90}
          endAngle={90 - (360 * Math.max(magnitude, 0.5) / 100)}
          innerRadius={86}
          outerRadius={120}
          style={{ width: "100%", height: "100%" }}
        >
          <PolarGrid
            gridType="circle"
            radialLines={false}
            stroke="none"
            polarRadii={[92, 80]}
            className="first:fill-muted/30 last:fill-background/60"
          />
          <RadialBar
            dataKey="value"
            background={false}
            cornerRadius={10}
            isAnimationActive
            animationDuration={500}
          />
          <PolarRadiusAxis tick={false} tickLine={false} axisLine={false}>
            <Label
              content={({ viewBox }) => {
                if (!viewBox || !("cx" in viewBox) || !("cy" in viewBox)) return null;
                const cx = viewBox.cx ?? 0;
                const cy = viewBox.cy ?? 0;
                return (
                  <text
                    x={cx}
                    y={cy}
                    textAnchor="middle"
                    dominantBaseline="middle"
                  >
                    <tspan
                      x={cx}
                      y={cy - 6}
                      fill={accent.cssColor}
                      className="ext-num font-semibold tabular-nums"
                      style={{ fontSize: valueFontSize, fontWeight: 600 }}
                    >
                      {value}
                    </tspan>
                    {unit ? (
                      <tspan
                        x={cx}
                        y={cy + 18}
                        className="fill-muted-foreground"
                        style={{ fontSize: 12 }}
                      >
                        {unit}
                      </tspan>
                    ) : null}
                  </text>
                );
              }}
            />
          </PolarRadiusAxis>
        </RadialBarChart>
      </div>

      {/* ── Footer ───────────────────────────────────── */}
      <footer className="flex items-center justify-center gap-2 text-sm border-t border-border/30 pt-3">
        {deltaPct !== null && Number.isFinite(deltaPct) ? (
          <>
            <span
              className={
                "inline-flex items-center gap-1 px-2 py-0.5 text-xs tabular-nums " +
                "rounded-full border font-medium " +
                (neutral
                  ? "text-muted-foreground border-border/40 bg-muted/20"
                  : goodNews
                    ? "text-emerald-300 border-emerald-400/40 bg-emerald-400/10"
                    : "text-amber-300 border-amber-400/40 bg-amber-400/10")
              }
            >
              <span aria-hidden="true">{arrow}</span>
              {Math.abs(deltaPct).toFixed(1)}%
            </span>
            <span className="text-xs text-muted-foreground inline-flex items-center gap-1">
              <IconTrend size={12} />
              {deltaLabel ?? "vs prior period"}
            </span>
          </>
        ) : (
          <span className="text-xs text-muted-foreground italic">
            no prior period data
          </span>
        )}
      </footer>
    </div>
  );
}

// Hero-number font size that scales down for long values so an
// 11-char "164,178.9B" fits inside the radial well.
function pickFontSize(len: number): number {
  if (len <= 4)  return 44;
  if (len <= 6)  return 36;
  if (len <= 8)  return 30;
  if (len <= 10) return 24;
  return 20;
}
