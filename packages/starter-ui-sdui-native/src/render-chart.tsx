// `chart` — text-summary fallback on mobile. The native rich chart
// (react-native-svg + polyline) will live in
// `@nube/starter-ui-dashboard-native/performance-chart.tsx` (next
// stage); the renderer here delegates the visual to a host-supplied
// custom renderer registered against the same `customRenderers`
// registry, and falls back to a tagged Card summarising the series.
//
// Note: per spec, `sparkline` is in the deferred-with-web set and
// is NOT aliased here (the web alias is web-only).
import type { UiComponent } from "@nube/starter-ui-ir";
import { Card, CardContent, CardHeader, CardTitle, Column, Text } from "@nube/starter-ui-kit-native";
import { registerRenderer } from "@nube/starter-ui-sdui-react/headless";

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

function extractSeries(node: UiComponent): Point[][] {
  if (Array.isArray(node.sources)) {
    return node.sources.map(extractPoints).filter((s) => s.length > 0);
  }
  if (Array.isArray(node.series)) {
    return node.series
      .map((s) =>
        s && typeof s === "object" && Array.isArray((s as { points?: unknown }).points)
          ? extractPoints(s)
          : [],
      )
      .filter((s) => s.length > 0);
  }
  return [];
}

function summary(series: Point[][]): string {
  if (series.length === 0) return "no data";
  const total = series.reduce((n, s) => n + s.length, 0);
  return `${series.length} series · ${total} points`;
}

export function RenderChart({ node }: { node: UiComponent }) {
  const title = typeof node.title === "string" ? node.title : "Chart";
  const series = extractSeries(node);
  return (
    <Card
      accessibilityRole="image"
      accessibilityLabel={title}
      testID={(node.id as string | undefined) ?? "sdui-chart"}
    >
      <CardHeader>
        <CardTitle>
          <Text variant="label" weight="medium">
            {title}
          </Text>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <Column gap={4} testID={`${node.id ?? "sdui-chart"}-summary`}>
          <Text variant="caption" color="muted">
            {summary(series)}
          </Text>
        </Column>
      </CardContent>
    </Card>
  );
}

registerRenderer("chart", RenderChart);
