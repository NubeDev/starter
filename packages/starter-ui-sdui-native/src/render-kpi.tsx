// `kpi` — single big-number card with label + optional unit + trend.
// RN port of `starter-ui-sdui-react/src/renderer/render-kpi.tsx`.
// Reads from `node.value` first, then from `node.source.points`
// (last numeric point), so static fixtures like
// `crates/rubix-flows/dashboards/disk-overview.json` work without
// a transport.
//
// Visual contract: see `rubix/docs/design/sdui/visual-design-spec.md`.
// The web renderer's glass + blur effect is approximated on RN with a
// 2px accent border on top of the card; the KPI value is tinted in
// the accent color and rendered with tabular figures.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Box, Card, CardContent, Column, Row, Text, useTheme } from "@nube/starter-ui-kit-native";
import { registerRenderer, resolveAccent } from "@nube/starter-ui-sdui-react/headless";
import { accentHex, trendColor } from "./accent-colors.js";

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
  const theme = useTheme();
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
  const accent = resolveAccent(node as Record<string, unknown>);
  const accentColor = accentHex(accent, theme.mode);
  const tColor = trendColor(trend, theme.mode);
  // The web renderer paints a hairline gradient on top of the card;
  // RN's Card primitive doesn't accept `style`, so we wrap it in a
  // Box and paint a 2px accent strip above. Accent also tints the
  // value text — same visual cue without a custom surface fork.
  return (
    <Box>
      <Box
        style={{
          height: 2,
          backgroundColor: accentColor,
          borderTopLeftRadius: 16,
          borderTopRightRadius: 16,
          marginBottom: -1,
        }}
      />
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
              <Text
                variant="title"
                weight="semibold"
                style={{
                  color: accentColor,
                  fontVariant: ["tabular-nums"],
                }}
              >
                {value}
              </Text>
              {unit ? (
                <Text variant="label" color="muted">
                  {unit}
                </Text>
              ) : null}
            </Row>
            {trend ? (
              <Text
                variant="caption"
                color="muted"
                style={tColor ? { color: tColor } : undefined}
              >
                {trend}
              </Text>
            ) : null}
          </Column>
        </CardContent>
      </Card>
    </Box>
  );
}

registerRenderer("kpi", RenderKpi);
