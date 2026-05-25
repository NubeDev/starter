// `kpi` — single big-number card with label + optional unit + trend.
// RN port of `starter-ui-sdui-react/src/renderer/render-kpi.tsx`.
// Reads from `node.value` first, then from `node.source.points`
// (last numeric point), so static fixtures like
// `crates/rubix-flows/dashboards/disk-overview.json` work without
// a transport.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Card, CardContent, Column, Row, Text } from "@nube/starter-ui-kit-native";
import { registerRenderer } from "@nube/starter-ui-sdui-react/headless";

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

export function RenderKpi({ node }: { node: UiComponent }) {
  const label = typeof node.label === "string" ? node.label : "";
  const sourced = lastNumericPoint(node.source);
  const raw =
    typeof node.value === "number" || typeof node.value === "string"
      ? node.value
      : sourced;
  const value =
    raw === undefined
      ? "—"
      : formatValue(
          raw,
          typeof node.format === "string" ? node.format : undefined,
        );
  const unit =
    typeof node.unit_symbol === "string"
      ? node.unit_symbol
      : typeof node.unit === "string"
        ? node.unit
        : undefined;
  const trend = typeof node.trend === "string" ? node.trend : undefined;
  return (
    <Card
      accessibilityRole="summary"
      accessibilityLabel={label ? `${label}: ${value}${unit ? ` ${unit}` : ""}` : undefined}
      testID={(node.id as string | undefined) ?? "sdui-kpi"}
    >
      <CardContent>
        <Column gap={4}>
          <Text variant="caption" color="muted">
            {label}
          </Text>
          <Row gap={4}>
            <Text variant="title" weight="semibold">
              {value}
            </Text>
            {unit ? (
              <Text variant="label" color="muted">
                {unit}
              </Text>
            ) : null}
          </Row>
          {trend ? (
            <Text variant="caption" color="muted">
              {trend}
            </Text>
          ) : null}
        </Column>
      </CardContent>
    </Card>
  );
}

registerRenderer("kpi", RenderKpi);
