import { useMemo } from "react";
import { gaugeValue, jitter } from "@/data/fake";
import { useTick, useAnimatedNumber } from "@/lib/hooks";
import type { Widget } from "@/data/types";

// Custom SVG radial gauge — 270° sweep, threshold-aware colour, live needle.
export function GaugeWidget({ widget }: { widget: Widget }) {
  const tick = useTick(2600);
  const { min = 0, max = 100, warn, crit, unit, color = "152 76% 44%", decimals = 0 } =
    widget.config;

  const raw = useMemo(() => {
    const base = gaugeValue(widget.config.metric, min, max);
    return jitter(base, 0.05);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [widget.config.metric, tick, min, max]);

  const value = useAnimatedNumber(raw, 700);
  const pct = clamp((value - min) / (max - min), 0, 1);

  // Threshold colour: crit/warn may be ascending (load) or descending (battery).
  const state = thresholdState(value, warn, crit);
  const stroke =
    state === "crit"
      ? "0 72% 55%"
      : state === "warn"
        ? "38 95% 56%"
        : color;

  const size = 168;
  const cx = size / 2;
  const cy = size / 2;
  const r = 64;
  const startAngle = 135;
  const sweep = 270;
  const arc = (frac: number) => describeArc(cx, cy, r, startAngle, startAngle + sweep * frac);
  const needleAngle = (startAngle + sweep * pct) * (Math.PI / 180);
  const nx = cx + Math.cos(needleAngle) * (r - 10);
  const ny = cy + Math.sin(needleAngle) * (r - 10);

  return (
    <div className="flex h-full w-full flex-col items-center justify-center">
      <div className="relative">
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
          <path d={arc(1)} fill="none" stroke="hsl(217 33% 16%)" strokeWidth={12} strokeLinecap="round" />
          <path
            d={arc(pct)}
            fill="none"
            stroke={`hsl(${stroke})`}
            strokeWidth={12}
            strokeLinecap="round"
            style={{ filter: `drop-shadow(0 0 8px hsl(${stroke} / 0.55))`, transition: "stroke 0.4s ease" }}
          />
          <circle cx={cx} cy={cy} r={5} fill={`hsl(${stroke})`} />
          <line x1={cx} y1={cy} x2={nx} y2={ny} stroke={`hsl(${stroke})`} strokeWidth={3} strokeLinecap="round" />
        </svg>
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center pb-2">
          <div className="tabular text-3xl font-bold leading-none text-foreground">
            {value.toFixed(decimals)}
          </div>
          {unit ? <div className="mt-1 text-xs font-medium text-muted-foreground">{unit}</div> : null}
        </div>
      </div>
      <div className="mt-1 flex w-full items-center justify-between px-6 text-[0.7rem] tabular text-muted-foreground">
        <span>{min}</span>
        <span className="capitalize" style={{ color: `hsl(${stroke})` }}>
          {state === "ok" ? "nominal" : state}
        </span>
        <span>{max}</span>
      </div>
    </div>
  );
}

function thresholdState(v: number, warn?: number, crit?: number): "ok" | "warn" | "crit" {
  if (warn == null || crit == null) return "ok";
  const descending = crit < warn; // e.g. battery SoC: crit 15 < warn 35
  if (descending) {
    if (v <= crit) return "crit";
    if (v <= warn) return "warn";
    return "ok";
  }
  if (v >= crit) return "crit";
  if (v >= warn) return "warn";
  return "ok";
}

function clamp(v: number, lo: number, hi: number) {
  return Math.min(hi, Math.max(lo, v));
}

function polar(cx: number, cy: number, r: number, deg: number) {
  const a = (deg * Math.PI) / 180;
  return { x: cx + r * Math.cos(a), y: cy + r * Math.sin(a) };
}

function describeArc(cx: number, cy: number, r: number, start: number, end: number) {
  const s = polar(cx, cy, r, start);
  const e = polar(cx, cy, r, end);
  const large = end - start <= 180 ? 0 : 1;
  return `M ${s.x} ${s.y} A ${r} ${r} 0 ${large} 1 ${e.x} ${e.y}`;
}
