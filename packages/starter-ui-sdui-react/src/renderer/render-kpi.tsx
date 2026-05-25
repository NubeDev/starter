// `kpi` — single big-number card with label + optional unit + trend.
import { Card, CardContent, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "./registry.js";

export function RenderKpi({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const label = typeof node.label === "string" ? node.label : "";
  const value =
    typeof node.value === "number" || typeof node.value === "string"
      ? String(node.value)
      : "—";
  const unit = typeof node.unit === "string" ? node.unit : undefined;
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
