// `chart` / `sparkline` — minimal inline SVG line chart over
// `node.sources[*].points` (the IR shape the backend ships). Falls
// back to `node.series` for legacy payloads. Consumers can override
// with a richer chart library via the `customRenderers` registry.
import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";

type Point = [number, number];

function extractPoints(source: unknown): Point[] {
  if (!source || typeof source !== "object") return [];
  const points = (source as { points?: unknown }).points;
  if (!Array.isArray(points)) return [];
  return points.filter(
    (p): p is Point =>
      Array.isArray(p) && p.length === 2 && typeof p[0] === "number" && typeof p[1] === "number",
  );
}

function extractSeries(node: import("@nube/starter-ui-ir").UiComponent): Point[][] {
  // Server-emitted `series` is authoritative — the chart-source
  // resolver fills it from `Static` / `AnalyticsTemplate` /
  // telemetry sources. Only fall back to inline `sources[*].points`
  // when the server didn't emit any series (legacy authored
  // dashboards that ship inline static points without a resolver).
  if (Array.isArray(node.series)) {
    const fromSeries = node.series
      .map((s) =>
        s && typeof s === "object" && Array.isArray((s as { points?: unknown }).points)
          ? extractPoints(s)
          : [],
      )
      .filter((s) => s.length > 0);
    if (fromSeries.length > 0) return fromSeries;
  }
  if (Array.isArray(node.sources)) {
    return node.sources.map(extractPoints).filter((s) => s.length > 0);
  }
  return [];
}

const PALETTE = ["#2563eb", "#16a34a", "#dc2626", "#ca8a04", "#9333ea"];
const WIDTH = 600;
const HEIGHT = 160;
const PAD = 8;

function toPath(points: Point[], xMin: number, xMax: number, yMin: number, yMax: number): string {
  if (points.length === 0) return "";
  const xSpan = xMax - xMin || 1;
  const ySpan = yMax - yMin || 1;
  const innerW = WIDTH - PAD * 2;
  const innerH = HEIGHT - PAD * 2;
  return points
    .map(([x, y], i) => {
      const px = PAD + ((x - xMin) / xSpan) * innerW;
      const py = PAD + innerH - ((y - yMin) / ySpan) * innerH;
      return `${i === 0 ? "M" : "L"}${px.toFixed(1)},${py.toFixed(1)}`;
    })
    .join(" ");
}

export function RenderChart({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const title = typeof node.title === "string" ? node.title : "Chart";
  const series = useMemo(() => extractSeries(node), [node]);

  const bounds = useMemo(() => {
    let xMin = Infinity;
    let xMax = -Infinity;
    let yMin = Infinity;
    let yMax = -Infinity;
    for (const s of series) {
      for (const [x, y] of s) {
        if (x < xMin) xMin = x;
        if (x > xMax) xMax = x;
        if (y < yMin) yMin = y;
        if (y > yMax) yMax = y;
      }
    }
    if (yMin === yMax) {
      yMin -= 1;
      yMax += 1;
    }
    return { xMin, xMax, yMin, yMax };
  }, [series]);

  return (
    <Card className={cn("sdui-chart", node.style?.className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        {series.length === 0 ? (
          <div
            className="h-40 w-full rounded border border-dashed border-muted-foreground/40 flex items-center justify-center text-xs text-muted-foreground"
            data-sdui-chart-series-count={0}
          >
            no data
          </div>
        ) : (
          <svg
            viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
            preserveAspectRatio="none"
            className="h-40 w-full"
            data-sdui-chart-series-count={series.length}
          >
            {series.map((s, i) => (
              <path
                key={i}
                d={toPath(s, bounds.xMin, bounds.xMax, bounds.yMin, bounds.yMax)}
                fill="none"
                stroke={PALETTE[i % PALETTE.length]}
                strokeWidth={1.5}
                vectorEffect="non-scaling-stroke"
              />
            ))}
          </svg>
        )}
      </CardContent>
    </Card>
  );
}

registerRenderer("chart", RenderChart);
registerRenderer("sparkline", RenderChart);
