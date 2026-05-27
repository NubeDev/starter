// `chart` / `sparkline` — µPlot-backed line chart. Reads
// `node.series` (server-resolved) with fallback to inline
// `node.sources[*].points`. Each point is `[timestamp_ms, value]`.
import { useEffect, useMemo, useRef } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";

type Point = [number, number];
type Series = { label?: string; points: Point[] };

function extractPoints(source: unknown): Point[] {
  if (!source || typeof source !== "object") return [];
  const points = (source as { points?: unknown }).points;
  if (!Array.isArray(points)) return [];
  return points.filter(
    (p): p is Point =>
      Array.isArray(p) && p.length === 2 && typeof p[0] === "number" && typeof p[1] === "number",
  );
}

function extractLabel(source: unknown): string | undefined {
  if (!source || typeof source !== "object") return undefined;
  const label = (source as { label?: unknown }).label;
  return typeof label === "string" ? label : undefined;
}

function extractSeries(node: import("@nube/starter-ui-ir").UiComponent): Series[] {
  if (Array.isArray(node.series)) {
    const fromSeries = node.series
      .map((s) => ({ label: extractLabel(s), points: extractPoints(s) }))
      .filter((s) => s.points.length > 0);
    if (fromSeries.length > 0) return fromSeries;
  }
  if (Array.isArray(node.sources)) {
    return node.sources
      .map((s) => ({ label: extractLabel(s), points: extractPoints(s) }))
      .filter((s) => s.points.length > 0);
  }
  return [];
}

// Static fallback palette — used only if theme tokens fail to resolve
// (e.g. SSR or test env without computed styles).
const FALLBACK_PALETTE = ["#2563eb", "#16a34a", "#dc2626", "#ca8a04", "#9333ea"];

// Read the active rubix theme tokens off the host element. Live read
// means a palette/mode switch on the next chart construction picks up
// the new colors; series strokes within an existing chart instance
// stay frozen (acceptable — palette swaps are user-initiated and
// uncommon, and `series.length` change forces a rebuild anyway).
function readThemePalette(host: HTMLElement): {
  series: string[];
  axis: string;
  grid: string;
  text: string;
} {
  if (typeof window === "undefined") {
    return {
      series: FALLBACK_PALETTE,
      axis: "rgba(0,0,0,0.4)",
      grid: "rgba(0,0,0,0.08)",
      text: "rgba(0,0,0,0.6)",
    };
  }
  const cs = getComputedStyle(host);
  const get = (name: string): string | undefined => {
    const v = cs.getPropertyValue(name).trim();
    return v.length > 0 ? v : undefined;
  };
  const series = [
    get("--color-leaf"),
    get("--color-aqua"),
    get("--color-sun"),
    get("--color-sky"),
    get("--color-warn"),
  ].filter((c): c is string => !!c);
  return {
    series: series.length > 0 ? series : FALLBACK_PALETTE,
    axis: get("--color-muted") ?? "rgba(0,0,0,0.4)",
    grid: get("--color-border") ?? "rgba(0,0,0,0.08)",
    text: get("--color-muted") ?? "rgba(0,0,0,0.6)",
  };
}

// µPlot data shape: [xs, ys1, ys2, ...]. All series share the x
// axis (union of timestamps, sorted, in seconds).
function toUPlotData(series: Series[]): uPlot.AlignedData {
  const xs = new Set<number>();
  for (const s of series) for (const [x] of s.points) xs.add(x);
  let xAxis = [...xs].sort((a, b) => a - b);
  // µPlot's time scale needs ≥2 x values to draw anything.
  if (xAxis.length === 1) {
    const t = xAxis[0]!;
    xAxis = [t - 1000, t, t + 1000];
  }
  const xSeconds = xAxis.map((t) => t / 1000);
  const cols: (number[] | (number | null)[])[] = [xSeconds];
  for (const s of series) {
    const lookup = new Map<number, number>();
    for (const [x, y] of s.points) {
      if (Number.isFinite(y)) lookup.set(x, y);
    }
    cols.push(xAxis.map((x) => lookup.get(x) ?? null));
  }
  return cols as unknown as uPlot.AlignedData;
}

function UPlotChart({
  series,
  height,
  showAxes,
  showLegend,
}: {
  series: Series[];
  height: number;
  showAxes: boolean;
  showLegend: boolean;
}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<uPlot | null>(null);
  // Stash latest series shape in a ref so the resize observer
  // doesn't need to live inside the data effect.
  const data = useMemo(() => toUPlotData(series), [series]);

  // Construct chart exactly once per mount. We don't pass `series`
  // as a dep — µPlot is updated in place via `setData` below. This
  // avoids the destroy/recreate cycle that left blank canvases on
  // subscription pushes in earlier attempts.
  useEffect(() => {
    const host = hostRef.current;
    if (!host || series.length === 0) return;

    // µPlot's constructor uses `target instanceof HTMLElement` to
    // decide between appending the chart root and invoking a
    // ready-callback. That check runs against µPlot's *own* module
    // global (the parent window's HTMLElement). When the host lives
    // inside an iframe (Puck v0.19 preview), it's an instance of the
    // iframe's HTMLElement — a different class — and the check
    // fails, so µPlot treats our DOM node as a callback and crashes
    // with "then is not a function". Compare ownerDocument against
    // the module-global document to detect this.
    const inIframe =
      typeof document === "undefined" || host.ownerDocument !== document;

    // Measure the host. If layout hasn't settled yet, fall back to
    // 600 — the ResizeObserver below will fix this up immediately.
    const initialWidth = host.clientWidth > 0 ? host.clientWidth : 600;

    const palette = readThemePalette(host);

    const opts: uPlot.Options = {
      width: initialWidth,
      height,
      scales: { x: { time: true } },
      cursor: { drag: { x: true, y: false } },
      legend: { show: showLegend },
      axes: showAxes
        ? [
            {
              stroke: palette.axis,
              grid: { stroke: palette.grid, width: 1 },
              ticks: { stroke: palette.grid, width: 1, size: 6 },
              font: '11px var(--font-sans, system-ui)',
            },
            {
              stroke: palette.axis,
              grid: { stroke: palette.grid, width: 1 },
              ticks: { stroke: palette.grid, width: 1, size: 6 },
              font: '11px var(--font-sans, system-ui)',
            },
          ]
        : [{ show: false }, { show: false }],
      series: [
        {},
        ...series.map((s, i) => {
          const color = palette.series[i % palette.series.length]!;
          return {
            label: s.label ?? `series ${i + 1}`,
            stroke: color,
            width: 2,
            fill: `color-mix(in oklab, ${color} 14%, transparent)`,
            points: { show: false },
          };
        }),
      ],
    };

    const chart = inIframe
      ? new uPlot(opts, data, (self) => {
          host.appendChild(self.root);
        })
      : new uPlot(opts, data, host);
    chartRef.current = chart;

    // If we initialised with the fallback width, sync to real width
    // synchronously so the first paint isn't wrong.
    if (host.clientWidth > 0 && host.clientWidth !== initialWidth) {
      chart.setSize({ width: host.clientWidth, height });
    }

    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? 0;
      if (w > 0 && chartRef.current) {
        chartRef.current.setSize({ width: w, height });
      }
    });
    ro.observe(host);

    return () => {
      ro.disconnect();
      chart.destroy();
      chartRef.current = null;
    };
    // Intentionally exclude `data`, `series`, `showLegend`, `showAxes`
    // — they're handled by the in-place update effect below. Only
    // structural changes (series count, height) force a rebuild,
    // covered via `seriesCount`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [series.length, height]);

  // Push new data into the existing chart on every series change.
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;
    chart.setData(data);
  }, [data]);

  return <div ref={hostRef} style={{ width: "100%", height }} data-sdui-chart-series-count={series.length} />;
}

export function RenderChart({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const title = typeof node.title === "string" ? node.title : "Chart";
  const series = useMemo(() => extractSeries(node), [node]);
  const isSparkline = node.type === "sparkline";
  const height = isSparkline ? 64 : 180;

  return (
    <div
      className={cn(
        "sdui-chart glass relative overflow-hidden rounded-3xl p-5 sm:p-6",
        node.style?.className,
      )}
    >
      <div className="mb-3 flex items-center justify-between gap-3">
        <h3 className="text-sm font-semibold tracking-[-0.01em] text-[color:var(--color-text)]">
          {title}
        </h3>
      </div>
      {series.length === 0 ? (
        <div
          className={cn(
            "w-full rounded-2xl border border-dashed flex items-center justify-center text-xs",
            isSparkline ? "h-16" : "h-44",
          )}
          style={{
            borderColor: "color-mix(in oklab, var(--color-muted) 40%, transparent)",
            color: "var(--color-muted)",
          }}
          data-sdui-chart-series-count={0}
        >
          no data
        </div>
      ) : (
        <UPlotChart
          series={series}
          height={height}
          showAxes={!isSparkline}
          showLegend={series.length > 1}
        />
      )}
    </div>
  );
}

registerRenderer("chart", RenderChart);
registerRenderer("sparkline", RenderChart);
