/**
 * `kpi` / `kpi_grid` — KPI tile + responsive grid wrapper. The KPI
 * carries a `label`, a `value`, an optional `unit`, an optional
 * trend delta, and an optional click action. The grid is a thin
 * shadcn-Card grid that arranges N KPIs across 1–4 responsive
 * columns.
 */
import { Card, CardContent } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import { Renderer } from "../Renderer.js";
import { useSdui } from "../context.js";
import type { UiComponent } from "../types.js";

export interface KpiNode extends UiComponent {
  type: "kpi";
  label: string;
  value: string | number;
  unit?: string;
  /** Optional trend: positive ↑ green, negative ↓ red. */
  delta?: number;
  /** Optional action handler — fires on tile click. */
  on_click?: string;
}

export const kpiSpec: ComponentSpec<KpiNode> = {
  kind: "kpi",
  Component: ({ node }) => {
    const { dispatchAction } = useSdui();
    const delta = node.delta;
    const onClick = node.on_click
      ? () => {
          void dispatchAction(node.on_click!);
        }
      : undefined;
    return (
      <Card
        onClick={onClick}
        className={`${onClick ? "cursor-pointer transition hover:bg-accent" : ""} ${
          node.style?.className ?? ""
        }`}
      >
        <CardContent className="flex flex-col gap-1 py-4">
          <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {node.label}
          </div>
          <div className="flex items-baseline gap-2">
            <div className="text-2xl font-semibold tabular-nums">{node.value}</div>
            {node.unit ? (
              <div className="text-sm text-muted-foreground">{node.unit}</div>
            ) : null}
          </div>
          {delta !== undefined ? (
            <div
              className={`text-xs ${
                delta > 0
                  ? "text-emerald-600"
                  : delta < 0
                  ? "text-destructive"
                  : "text-muted-foreground"
              }`}
            >
              {delta > 0 ? "▲" : delta < 0 ? "▼" : "—"} {Math.abs(delta)}
            </div>
          ) : null}
        </CardContent>
      </Card>
    );
  },
};

export interface KpiGridNode extends UiComponent {
  type: "kpi_grid";
  cols?: 1 | 2 | 3 | 4;
  children: UiComponent[];
}

const KPI_COLS: Record<number, string> = {
  1: "grid-cols-1",
  2: "grid-cols-1 sm:grid-cols-2",
  3: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3",
  4: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-4",
};

export const kpiGridSpec: ComponentSpec<KpiGridNode> = {
  kind: "kpi_grid",
  Component: ({ node }) => (
    <div className={`grid gap-4 ${KPI_COLS[node.cols ?? 4]}`}>
      {(node.children ?? []).map((c, i) => (
        <Renderer key={c.id ?? i} node={c} />
      ))}
    </div>
  ),
};
