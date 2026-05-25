// `chart` / `sparkline` — chart placeholder. v1 renders a labelled
// frame with the series JSON in a `<pre>`; the rubix frontend can
// swap this out via a custom renderer once the chart kit lands.
import { Card, CardContent, CardHeader, CardTitle, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "./registry.js";

export function RenderChart({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const title = typeof node.title === "string" ? node.title : "Chart";
  const series = Array.isArray(node.series) ? node.series : [];
  return (
    <Card className={cn("sdui-chart", node.style?.className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <div
          className="h-40 w-full rounded border border-dashed border-muted-foreground/40 flex items-center justify-center text-xs text-muted-foreground"
          data-sdui-chart-series-count={series.length}
        >
          chart placeholder — {series.length} series
        </div>
      </CardContent>
    </Card>
  );
}

registerRenderer("chart", RenderChart);
registerRenderer("sparkline", RenderChart);
