// `kpi_grid` — responsive grid of pre-resolved KPI tiles. The IR
// shape is `{ items: KpiGridItem[], columns?: number }`; each item
// inlines `label`, `value`, `unit_symbol`, optional `delta` and
// `on_click`. Distinct from `grid` of `kpi` nodes because the
// resolver pre-flattens here and the renderer only deals with
// scalar tiles — no per-tile transport calls.
import { cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { accentByIndex, accentVar, resolveAccent } from "./accent.js";

interface KpiGridItem {
  id?: string;
  label: string;
  value: unknown;
  unit_symbol?: string;
  format?: string;
  intent?: string;
  accent?: string;
  delta?: { value?: number; direction?: string; label?: string };
}

function formatValue(raw: unknown, format: string | undefined): string {
  if (raw == null) return "—";
  if (typeof raw === "string") return raw;
  if (typeof raw === "number") {
    if (format === "percent") return raw.toFixed(raw % 1 === 0 ? 0 : 1);
    if (format === "number") return raw.toLocaleString();
    return String(raw);
  }
  return String(raw);
}

function deltaColor(direction: string | undefined): string | undefined {
  if (direction === "up") return "var(--color-ok)";
  if (direction === "down") return "var(--color-danger)";
  return undefined;
}

export function RenderKpiGrid({
  node,
}: {
  node: import("@nube/starter-ui-ir").UiComponent;
}) {
  const items: KpiGridItem[] = Array.isArray(node.items)
    ? (node.items as KpiGridItem[]).filter(
        (i) => i && typeof i === "object" && typeof i.label === "string",
      )
    : [];
  const cols = typeof node.columns === "number" && node.columns > 0 ? node.columns : 4;
  const gridStyle: React.CSSProperties = {
    display: "grid",
    gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`,
    gap: "1rem",
  };
  return (
    <div
      className={cn("sdui-kpi-grid", node.style?.className)}
      style={gridStyle}
      data-sdui-kpi-grid-cols={cols}
    >
      {items.map((item, i) => {
        const accent = item.accent || item.intent ? resolveAccent(item) : accentByIndex(i);
        const c = accentVar(accent);
        const dC = deltaColor(item.delta?.direction);
        return (
          <div
            key={item.id ?? `${item.label}:${i}`}
            className="sdui-kpi-grid-item glass relative overflow-hidden rounded-3xl p-5"
            data-sdui-accent={accent}
          >
            <div
              aria-hidden
              className="pointer-events-none absolute -right-10 -top-10 h-28 w-28 rounded-full opacity-40 blur-2xl"
              style={{ background: `color-mix(in oklab, ${c} 55%, transparent)` }}
            />
            <div
              aria-hidden
              className="pointer-events-none absolute inset-x-5 top-0 h-px"
              style={{ background: `linear-gradient(90deg, transparent, ${c}, transparent)` }}
            />
            <div className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
              {item.label}
            </div>
            <div className="mt-2 flex items-baseline gap-1.5">
              <span
                className="tabular font-medium tracking-[-0.03em] text-3xl sm:text-4xl"
                style={{ color: c }}
              >
                {formatValue(item.value, item.format)}
              </span>
              {item.unit_symbol ? (
                <span className="text-sm font-medium text-[color:var(--color-muted)]">{item.unit_symbol}</span>
              ) : null}
            </div>
            {item.delta?.label ? (
              <div
                className="mt-2 inline-flex items-center gap-1 text-xs font-medium text-[color:var(--color-muted)]"
                style={dC ? { color: dC } : undefined}
              >
                {item.delta.label}
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

registerRenderer("kpi_grid", RenderKpiGrid);
