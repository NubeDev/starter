/**
 * `chart` — multi-series time-series chart. The IR carries
 * `sources: ChartSource[]` (V5 multi-series; the single legacy
 * `source` field is folded into a one-element vector by the wire
 * adapter), a `kind: "line" | "bar" | "area"`, a server-baked
 * `series` payload (one per source), an optional visible `range`,
 * and the `page_state_key` (default `"chart_range"`) the client
 * writes zoom / pan state into.
 *
 * Per SCOPE.md, writing `$page[page_state_key]` triggers a
 * re-resolve with the new range — the round-trip stays
 * server-authoritative (R9 — no client-side business logic). This
 * spec writes the new range through `setPageState`; the host's
 * SduiPage re-issues the resolve request whose body carries
 * `page_state`, and the server returns a fresh `series` payload.
 *
 * The wrapper is intentionally a thin SVG sketch — production
 * consumers swap in a heavy chart library through a `custom`
 * renderer when they need real interaction. The IR contract is
 * what matters: source / kind / range / page_state_key shape.
 */
import { useCallback, useMemo } from "react";
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";
import { useSdui } from "../context.js";

export interface ChartPoint {
  ts: number;
  value: number;
}
export interface ChartSeries {
  id?: string;
  label?: string;
  points: ChartPoint[];
}
export interface ChartRange {
  from?: number | null;
  to?: number | null;
}
export interface ChartNode extends UiComponent {
  type: "chart";
  kind?: "line" | "bar" | "area";
  /** V5 multi-series — preferred field. */
  sources?: unknown[];
  /** Legacy single-source dual-field tolerance (V4 → V5). */
  source?: unknown;
  series?: ChartSeries[];
  range?: ChartRange;
  /** Default `"chart_range"` per IR; the round-trip key. */
  page_state_key?: string;
}

function flattenSeries(series: ChartSeries[]): ChartPoint[] {
  const out: ChartPoint[] = [];
  for (const s of series) for (const p of s.points) out.push(p);
  return out;
}

function bounds(points: ChartPoint[]): { minT: number; maxT: number; minV: number; maxV: number } {
  if (points.length === 0) return { minT: 0, maxT: 1, minV: 0, maxV: 1 };
  let minT = Infinity, maxT = -Infinity, minV = Infinity, maxV = -Infinity;
  for (const p of points) {
    if (p.ts < minT) minT = p.ts;
    if (p.ts > maxT) maxT = p.ts;
    if (p.value < minV) minV = p.value;
    if (p.value > maxV) maxV = p.value;
  }
  if (minT === maxT) maxT = minT + 1;
  if (minV === maxV) maxV = minV + 1;
  return { minT, maxT, minV, maxV };
}

export const chartSpec: ComponentSpec<ChartNode> = {
  kind: "chart" as never,
  Component: ({ node }) => {
    const { pageState, setPageState } = useSdui();
    const stateKey = node.page_state_key ?? "chart_range";

    // V5 dual-field tolerance — if a legacy `source` snuck through
    // without a `sources` vector, treat it as a single-element list.
    // The actual rendering only consumes the server-emitted
    // `series` payload; sources/source affect the resolver, not the
    // pixel layer.
    void (Array.isArray(node.sources) ? node.sources : node.source ? [node.source] : []);

    const series = node.series ?? [];
    const flat = useMemo(() => flattenSeries(series), [series]);
    const { minT, maxT, minV, maxV } = useMemo(() => bounds(flat), [flat]);

    const width = 600;
    const height = 180;
    const pad = 16;

    const project = useCallback(
      (p: ChartPoint) => {
        const x = pad + ((p.ts - minT) / (maxT - minT)) * (width - 2 * pad);
        const y = height - pad - ((p.value - minV) / (maxV - minV)) * (height - 2 * pad);
        return { x, y };
      },
      [minT, maxT, minV, maxV],
    );

    // Zoom / pan round-trip — write a new range under `stateKey`
    // and the host's resolve query re-runs with the new window.
    // R9: no client-side recomputation; the server returns fresh
    // points.
    const setRange = useCallback(
      (range: ChartRange) => {
        setPageState({ [stateKey]: range });
      },
      [setPageState, stateKey],
    );

    // Click-to-reset — clears the page-state range so the server
    // falls back to the IR-default window on the next resolve.
    const reset = useCallback(() => {
      setPageState({ [stateKey]: null });
    }, [setPageState, stateKey]);

    // Pinch / drag is host-territory; this thin wrapper exposes the
    // round-trip via two buttons so the contract is observable in
    // tests without pulling a chart library.
    const current = (pageState[stateKey] as ChartRange | null | undefined) ?? node.range;

    const kind = node.kind ?? "line";

    return (
      <div className={`flex flex-col gap-2 ${node.style?.className ?? ""}`}>
        <svg
          width="100%"
          viewBox={`0 0 ${width} ${height}`}
          role="img"
          aria-label={`${kind} chart with ${series.length} series`}
        >
          {series.map((s, si) => {
            const pts = s.points.map(project);
            const d = pts.map((p, i) => `${i === 0 ? "M" : "L"}${p.x},${p.y}`).join(" ");
            return (
              <g key={s.id ?? si}>
                {kind === "bar"
                  ? pts.map((p, i) => (
                      <rect
                        key={i}
                        x={p.x - 2}
                        y={p.y}
                        width="4"
                        height={height - pad - p.y}
                        className="fill-primary/70"
                      />
                    ))
                  : (
                    <path
                      d={d}
                      className={
                        kind === "area"
                          ? "fill-primary/20 stroke-primary"
                          : "fill-none stroke-primary"
                      }
                      strokeWidth="1.5"
                    />
                  )}
              </g>
            );
          })}
        </svg>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <button
            type="button"
            className="rounded border px-2 py-1"
            onClick={() => setRange({ from: minT, to: maxT })}
          >
            zoom
          </button>
          <button
            type="button"
            className="rounded border px-2 py-1"
            onClick={reset}
          >
            reset
          </button>
          <span>
            range: {current?.from ?? "?"} – {current?.to ?? "?"}
          </span>
        </div>
      </div>
    );
  },
};

// ---------------------------------------------------------------------------
// `sparkline` — inline mini-chart, no axes, no interaction.
// Designed for KPI tiles. Optional subscribe extends the values
// vector live; mode is always "append" per IR.
// ---------------------------------------------------------------------------
export interface SparklineNode extends UiComponent {
  type: "sparkline";
  values?: number[];
  subscribe?: string;
  unit_symbol?: string;
  intent?: string;
}

export const sparklineSpec: ComponentSpec<SparklineNode> = {
  kind: "sparkline" as never,
  Component: ({ node }) => {
    const pts = node.values ?? [];
    if (pts.length === 0) {
      return <span className="text-xs text-muted-foreground">—</span>;
    }
    const min = Math.min(...pts);
    const max = Math.max(...pts) === min ? min + 1 : Math.max(...pts);
    const width = 80;
    const height = 24;
    const dx = width / Math.max(pts.length - 1, 1);
    const d = pts
      .map((v, i) => {
        const x = i * dx;
        const y = height - ((v - min) / (max - min)) * height;
        return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
    return (
      <svg
        width={width}
        height={height}
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label={`sparkline ${pts[pts.length - 1]}${node.unit_symbol ?? ""}`}
        className={node.style?.className}
      >
        <path d={d} className="fill-none stroke-primary" strokeWidth="1.25" />
      </svg>
    );
  },
};
