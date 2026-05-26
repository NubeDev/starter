// `kpi_grid` — responsive grid of pre-resolved KPI tiles. The IR
// shape is `{ items: KpiGridItem[], columns?: number }`; each item
// inlines `label`, `value`, `unit_symbol`, optional `delta` and
// `on_click`. Distinct from `grid` of `kpi` nodes because the
// resolver pre-flattens here and the renderer only deals with
// scalar tiles — no per-tile transport calls.
import {
  Card,
  CardContent,
  cn,
} from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";

interface KpiGridItem {
  id?: string;
  label: string;
  value: unknown;
  unit_symbol?: string;
  format?: string;
  intent?: string;
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
    gap: "0.75rem",
  };
  return (
    <div
      className={cn("sdui-kpi-grid", node.style?.className)}
      style={gridStyle}
      data-sdui-kpi-grid-cols={cols}
    >
      {items.map((item, i) => (
        <Card key={item.id ?? `${item.label}:${i}`} className="sdui-kpi-grid-item">
          <CardContent className="p-4">
            <div className="text-xs uppercase text-muted-foreground">{item.label}</div>
            <div className="mt-1 flex items-baseline gap-1">
              <span className="text-3xl font-semibold tabular-nums">
                {formatValue(item.value, item.format)}
              </span>
              {item.unit_symbol ? (
                <span className="text-sm text-muted-foreground">{item.unit_symbol}</span>
              ) : null}
            </div>
            {item.delta?.label ? (
              <div className="mt-1 text-xs text-muted-foreground">{item.delta.label}</div>
            ) : null}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

registerRenderer("kpi_grid", RenderKpiGrid);
