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
  /** Closed-set semantic colour. Applied to the card surface using
   *  theme-aware utility classes so light/dark both look right.
   *  Prefer this over `style.className` for colouring KPIs — it's the
   *  contract the prompt teaches the model and the only field the
   *  model is allowed to emit for colour. Free-form
   *  `style.className` is still honoured for edge cases. */
  intent?: "info" | "success" | "warning" | "danger" | "muted" | "accent";
}

/** Map closed-set `intent` token → Tailwind utility classes for the
 *  card surface. Foreground colour is left to the existing typography
 *  utilities (the label stays muted, the value stays the card's
 *  default text colour) so we don't fight contrast. */
const KPI_INTENT_CLASSES: Record<NonNullable<KpiNode["intent"]>, string> = {
  info: "border-sky-500/40 bg-sky-500/10",
  success: "border-emerald-500/40 bg-emerald-500/10",
  warning: "border-amber-500/40 bg-amber-500/10",
  danger: "border-rose-500/40 bg-rose-500/10",
  muted: "border-muted-foreground/20 bg-muted/40",
  accent: "border-violet-500/40 bg-violet-500/10",
};

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
    const intentClass = node.intent ? KPI_INTENT_CLASSES[node.intent] : "";
    return (
      <Card
        onClick={onClick}
        className={`${onClick ? "cursor-pointer transition hover:bg-accent" : ""} ${intentClass} ${
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
