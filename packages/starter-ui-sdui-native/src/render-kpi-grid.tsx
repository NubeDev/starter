// `kpi_grid` — responsive grid of pre-resolved KPI tiles. RN port of
// `starter-ui-sdui-react/src/renderer/render-kpi-grid.tsx`. The
// resolver pre-flattens each item to a scalar tile, so no per-tile
// transport calls.
import type { UiComponent } from "@nube/starter-ui-ir";
import {
  Box,
  Card,
  CardContent,
  Column,
  Row,
  Text,
  useTheme,
} from "@nube/starter-ui-kit-native";
import {
  accentByIndex,
  registerRenderer,
  resolveAccent,
} from "@nube/starter-ui-sdui-react/headless";
import { STATUS, accentHex } from "./accent-colors.js";

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

export function RenderKpiGrid({ node }: { node: UiComponent }) {
  const theme = useTheme();
  const items: KpiGridItem[] = Array.isArray(node.items)
    ? (node.items as KpiGridItem[]).filter(
        (i) => i && typeof i === "object" && typeof i.label === "string",
      )
    : [];
  if (items.length === 0) return null;

  // RN doesn't have a CSS grid primitive in the kit. Lay out items in
  // a wrapping Row; consumers can wrap us in a constrained-width Box
  // to control wrapping behaviour.
  return (
    <Box
      testID={(node.id as string | undefined) ?? "sdui-kpi-grid"}
      style={{ flexDirection: "row", flexWrap: "wrap", gap: 12 }}
    >
      {items.map((item, i) => {
        const explicit = !!item.accent || !!item.intent;
        const accent = explicit
          ? resolveAccent(item as unknown as Record<string, unknown>)
          : accentByIndex(i);
        const accentColor = accentHex(accent, theme.mode);
        const deltaColor =
          item.delta?.direction === "up"
            ? STATUS.ok[theme.mode]
            : item.delta?.direction === "down"
              ? STATUS.danger[theme.mode]
              : undefined;
        return (
          <Box
            key={item.id ?? `${item.label}:${i}`}
            style={{ flexGrow: 1, flexBasis: 140 }}
          >
            <Box
              style={{
                height: 2,
                backgroundColor: accentColor,
                borderTopLeftRadius: 16,
                borderTopRightRadius: 16,
                marginBottom: -1,
              }}
            />
            <Card testID={`sdui-kpi-grid-item-${i}`}>
              <CardContent>
                <Column gap={4}>
                  <Text variant="caption" color="muted">
                    {item.label}
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
                      {formatValue(item.value, item.format)}
                    </Text>
                    {item.unit_symbol ? (
                      <Text variant="label" color="muted">
                        {item.unit_symbol}
                      </Text>
                    ) : null}
                  </Row>
                  {item.delta?.label ? (
                    <Text
                      variant="caption"
                      color="muted"
                      style={deltaColor ? { color: deltaColor } : undefined}
                    >
                      {item.delta.label}
                    </Text>
                  ) : null}
                </Column>
              </CardContent>
            </Card>
          </Box>
        );
      })}
    </Box>
  );
}

registerRenderer("kpi_grid", RenderKpiGrid);
