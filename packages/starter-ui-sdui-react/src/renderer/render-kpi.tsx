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
import { cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { accentVar, resolveAccent } from "./accent.js";

type Point = [number, number];

function lastNumericPoint(source: unknown): number | undefined {
  if (!source || typeof source !== "object") return undefined;
  const s = source as { type?: unknown; points?: unknown; value?: unknown };
  const points = s.points;
  if (!Array.isArray(points) || points.length === 0) {
    warnOnMalformedStaticSource(s);
    return undefined;
  }
  const last = points[points.length - 1] as Point | undefined;
  if (!Array.isArray(last) || typeof last[1] !== "number") return undefined;
  return last[1];
}

let warnedKeys: Set<string> | undefined;
function warnOnMalformedStaticSource(s: {
  type?: unknown;
  points?: unknown;
  value?: unknown;
}): void {
  if (s.type !== "static") return;
  if (typeof console === "undefined") return;
  // Dedup so a chart re-render storm doesn't spam the console.
  const key = `static:${typeof s.value}:${Array.isArray(s.points) ? "empty-arr" : typeof s.points}`;
  warnedKeys ??= new Set();
  if (warnedKeys.has(key)) return;
  warnedKeys.add(key);
  if ("value" in s && typeof s.value !== "undefined") {
    console.warn(
      "[sdui] static source has `value` but no `points`; the renderer expects `{ type: 'static', points: [[ts_ms, value], ...] }`. The widget will render blank.",
    );
  } else {
    console.warn(
      "[sdui] static source has no `points` array; expected shape: `{ type: 'static', points: [[ts_ms, value], ...] }`.",
    );
  }
}

function formatValue(raw: number | string, format: string | undefined): string {
  if (typeof raw === "string") return raw;
  if (format === "percent") return raw.toFixed(raw % 1 === 0 ? 0 : 1);
  if (format === "number") return raw.toLocaleString();
  return String(raw);
}

function trendColor(trend: string | undefined): string | undefined {
  if (!trend) return undefined;
  if (trend.startsWith("+") || /^up\b/i.test(trend)) return "var(--color-ok)";
  if (trend.startsWith("-") || /^down\b/i.test(trend)) return "var(--color-danger)";
  return undefined;
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
  const accent = resolveAccent(node);
  const c = accentVar(accent);
  const trendC = trendColor(trend);
  return (
    <div
      className={cn(
        "sdui-kpi glass relative overflow-hidden rounded-3xl p-5 sm:p-6",
        node.style?.className,
      )}
      data-sdui-accent={accent}
    >
      <div
        aria-hidden
        className="pointer-events-none absolute -right-12 -top-12 h-32 w-32 rounded-full opacity-40 blur-2xl"
        style={{ background: `color-mix(in oklab, ${c} 55%, transparent)` }}
      />
      <div
        aria-hidden
        className="pointer-events-none absolute inset-x-5 top-0 h-px"
        style={{ background: `linear-gradient(90deg, transparent, ${c}, transparent)` }}
      />
      <div className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
        {label}
      </div>
      <div className="mt-3 flex items-baseline gap-1.5">
        <span
          className="tabular font-medium tracking-[-0.03em] text-[color:var(--color-text)] text-4xl sm:text-5xl"
          style={{ color: c }}
        >
          {value}
        </span>
        {unit ? (
          <span className="text-sm font-medium text-[color:var(--color-muted)]">{unit}</span>
        ) : null}
      </div>
      {trend ? (
        <div
          className="mt-2 inline-flex items-center gap-1 text-xs font-medium text-[color:var(--color-muted)]"
          style={trendC ? { color: trendC } : undefined}
        >
          {trend}
        </div>
      ) : null}
    </div>
  );
}

registerRenderer("kpi", RenderKpi);
