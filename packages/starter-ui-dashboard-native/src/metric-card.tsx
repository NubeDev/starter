// `<MetricCard>` — RN port of `starter-ui-dashboard/src/metric-card.tsx`.
// Prop API mirrors the web version one-to-one; the only thing that
// changes is the import path.
//
// Visual mapping:
//   - <motion.div>            → <MotiView>
//   - <div> + Tailwind         → kit-native `Card` + `Row`/`Column`
//   - inline <svg> sparkline   → `react-native-svg`
//   - Tailwind colour classes  → tokens via `useTheme()`

import * as React from "react";
import { MotiView } from "moti";
import {
  Defs,
  LinearGradient,
  Polygon,
  Polyline,
  Stop,
  Svg,
} from "react-native-svg";

import {
  Badge,
  Card,
  Column,
  Row,
  Text,
  useTheme,
} from "@nube/starter-ui-kit-native";

export interface MetricCardProps {
  /** Label rendered in the top-left (already localized). */
  label: string;
  /** Numeric value; animated on mount and on change. */
  value: number;
  /** Optional suffix appended after the animated number (e.g. "kWh"). */
  suffix?: string;
  /** Optional prefix prepended before the animated number (e.g. "$"). */
  prefix?: string;
  /** Percentage delta shown as a pill (e.g. `12.4` → "↑ 12.4%"). */
  delta?: number;
  /** Optional sparkline data; rendered as an inline area chart. */
  spark?: number[];
  /** Accent colour for the sparkline stroke. Defaults to the theme
   * foreground so the consumer's theme drives appearance. Pass any
   * CSS colour: `"#4ade80"`, `"hsl(var(--primary))"`, etc. */
  accent?: string;
  /** Reserved for parity with the web component. Ignored on RN since
   * style overrides should flow through `useTheme()` / `Card` props. */
  className?: string;
}

function Spark({
  data,
  color,
  width = 120,
  height = 36,
}: {
  data: number[];
  color: string;
  width?: number;
  height?: number;
}) {
  if (!data.length) return null;
  const max = Math.max(...data);
  const min = Math.min(...data);
  const range = max - min || 1;
  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1 || 1)) * width;
      const y = height - ((v - min) / range) * height;
      return `${x},${y}`;
    })
    .join(" ");
  const gradId = `spark-grad-${color.replace(/[^a-z0-9]/gi, "")}`;
  return (
    <Svg width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
      <Defs>
        <LinearGradient id={gradId} x1="0" x2="0" y1="0" y2="1">
          <Stop offset="0%" stopColor={color} stopOpacity="0.4" />
          <Stop offset="100%" stopColor={color} stopOpacity="0" />
        </LinearGradient>
      </Defs>
      <Polyline
        fill="none"
        stroke={color}
        strokeWidth={1.75}
        strokeLinecap="round"
        strokeLinejoin="round"
        points={points}
      />
      <Polygon
        points={`0,${height} ${points} ${width},${height}`}
        fill={`url(#${gradId})`}
      />
    </Svg>
  );
}

export function MetricCard({
  label,
  value,
  suffix,
  prefix,
  delta,
  spark = [],
  accent,
}: MetricCardProps): React.ReactElement {
  const t = useTheme();
  const accentColor = accent ?? t.color("foreground");
  const deltaPositive = (delta ?? 0) >= 0;
  const valueText = Math.round(value).toLocaleString();

  return (
    <MotiView
      from={{ opacity: 0, translateY: 20 }}
      animate={{ opacity: 1, translateY: 0 }}
      transition={{ type: "timing", duration: t.duration("slow") || 600 }}
    >
      <Card
        accessibilityRole="summary"
        accessibilityLabel={`${label}: ${prefix ?? ""}${valueText}${suffix ? ` ${suffix}` : ""}`}
      >
        <Row style={{ justifyContent: "space-between", alignItems: "flex-start" }}>
          <Text variant="caption" color="muted">
            {label}
          </Text>
          {typeof delta === "number" ? (
            <Badge variant={deltaPositive ? "default" : "destructive"}>
              {`${deltaPositive ? "↑" : "↓"} ${Math.abs(delta).toFixed(1)}%`}
            </Badge>
          ) : null}
        </Row>
        <Row style={{ justifyContent: "space-between", alignItems: "flex-end" }} gap={t.space(3)}>
          <Row gap={t.space(1)} style={{ alignItems: "baseline" }}>
            {prefix ? (
              <Text variant="subtitle" color="muted">
                {prefix}
              </Text>
            ) : null}
            <Text variant="title" weight="semibold">
              {valueText}
            </Text>
            {suffix ? (
              <Text variant="subtitle" color="muted">
                {suffix}
              </Text>
            ) : null}
          </Row>
          <Column>
            <Spark data={spark} color={accentColor} />
          </Column>
        </Row>
      </Card>
    </MotiView>
  );
}
