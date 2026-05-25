// `kpi` — single big-number card with label + optional unit + trend.
//
// Reads the value from either:
//   - `node.value` (server pre-resolved a scalar), or
//   - `node.source.points` ([[ts, value], …]) — the IR shape the
//     backend ships for static / live KPIs. Takes the last point.
//
// `node.format` (`percent` | `number` | …) drives display; the unit
// label comes from `node.unit_symbol` (current IR) or `node.unit`
// (legacy). See `crates/rubix-flows/dashboards/disk-overview.json`.
import { Card, CardContent, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "./registry.js";

type Point = [number, number];

function lastNumericPoint(source: unknown): number | undefined {
  if (!source || typeof source !== "object") return undefined;
  const points = (source as { points?: unknown }).points;
  if (!Array.isArray(points) || points.length === 0) return undefined;
  const last = points[points.length - 1] as Point | undefined;
  if (!Array.isArray(last) || typeof last[1] !== "number") return undefined;
  return last[1];
}

function formatValue(raw: number | string, format: string | undefined): string {
  if (typeof raw === "string") return raw;
  if (format === "percent") return raw.toFixed(raw % 1 === 0 ? 0 : 1);
  if (format === "number") return raw.toLocaleString();
  return String(raw);
}

export function RenderKpi({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const label = typeof node.label === "string" ? node.label : "";
  const sourced = lastNumericPoint(node.source);
  const raw =
    typeof node.value === "number" || typeof node.value === "string"
      ? node.value
      : sourced;
  const value = raw === undefined ? "—" : formatValue(raw, typeof node.format === "string" ? node.format : undefined);
  const unit =
    typeof node.unit_symbol === "string"
      ? node.unit_symbol
      : typeof node.unit === "string"
        ? node.unit
        : undefined;
  const trend = typeof node.trend === "string" ? node.trend : undefined;
  return (
    <Card className={cn("sdui-kpi", node.style?.className)}>
      <CardContent className="p-4">
        <div className="text-xs uppercase text-muted-foreground">{label}</div>
        <div className="mt-1 flex items-baseline gap-1">
          <span className="text-3xl font-semibold tabular-nums">{value}</span>
          {unit ? <span className="text-sm text-muted-foreground">{unit}</span> : null}
        </div>
        {trend ? <div className="mt-1 text-xs text-muted-foreground">{trend}</div> : null}
      </CardContent>
    </Card>
  );
}

registerRenderer("kpi", RenderKpi);
