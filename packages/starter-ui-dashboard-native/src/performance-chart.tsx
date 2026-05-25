// `<PerformanceChart>` — RN port of
// `starter-ui-dashboard/src/performance-chart.tsx`. Prop API mirrors
// the web version one-to-one (including the optional period selector
// and its `onPeriodChange` callback — the web `onClick` becomes
// `onPress` inside the kit `Button`, but the public callback signature
// is unchanged).
//
// The smoothed area chart uses `react-native-svg` paths; the fade-in
// of the line and area is driven by `MotiView` wrapping the `Svg`.

import * as React from "react";
import { MotiView } from "moti";
import {
  Defs,
  LinearGradient,
  Line,
  Path,
  Stop,
  Svg,
} from "react-native-svg";

import {
  Box,
  Button,
  Card,
  Column,
  Row,
  Text,
  useTheme,
} from "@nube/starter-ui-kit-native";

export interface PerformanceChartProps {
  /** Series values (one per labeled tick). At least 2 points needed
   * for a meaningful curve. */
  data: number[];
  /** X-axis tick labels. Render in the same order as `data`. */
  labels: string[];
  /** Section heading (already localized). */
  title: string;
  /** Optional headline value rendered next to the title (e.g. "42.3"). */
  headline?: string;
  /** Optional small suffix after the headline (e.g. "kWh"). */
  headlineSuffix?: string;
  /** Optional delta caption (e.g. "↑ 12.4%"). */
  delta?: string;
  /** Optional period selector items (e.g. `["1D","1W","1M","1Y"]`). */
  periods?: string[];
  /** 0-based index of the active period; ignored if `periods` is empty. */
  activePeriodIndex?: number;
  /** Called when a period is clicked. */
  onPeriodChange?: (index: number) => void;
  /** Stroke / accent colour for the line. Default theme `primary`. */
  accent?: string;
  /** Reserved for parity with the web component. Ignored on RN. */
  className?: string;
}

const W = 720;
const H = 240;
const PAD_X = 24;
const PAD_Y = 24;

function buildPath(points: ReadonlyArray<readonly [number, number]>): string {
  return points.reduce<string>((acc, point, i) => {
    const [x, y] = point;
    if (i === 0) return `M ${x} ${y}`;
    const prev = points[i - 1];
    if (!prev) return acc;
    const [px, py] = prev;
    const cx = (px + x) / 2;
    return `${acc} C ${cx} ${py}, ${cx} ${y}, ${x} ${y}`;
  }, "");
}

export function PerformanceChart({
  data,
  labels,
  title,
  headline,
  headlineSuffix,
  delta,
  periods,
  activePeriodIndex = 0,
  onPeriodChange,
  accent,
}: PerformanceChartProps): React.ReactElement {
  const t = useTheme();
  const stroke = accent ?? t.color("primary");

  const max = (data.length ? Math.max(...data) : 1) * 1.1;
  const min = 0;
  const range = max - min || 1;

  const points: Array<readonly [number, number]> = data.map((v, i) => {
    const x = PAD_X + (data.length > 1 ? (i / (data.length - 1)) * (W - PAD_X * 2) : 0);
    const y = H - PAD_Y - ((v - min) / range) * (H - PAD_Y * 2);
    return [x, y] as const;
  });

  const path = buildPath(points);
  const first = points[0];
  const last = points[points.length - 1];
  const area = first && last ? `${path} L ${last[0]} ${H - PAD_Y} L ${first[0]} ${H - PAD_Y} Z` : "";

  return (
    <Card
      accessibilityRole="image"
      accessibilityLabel={
        headline ? `${title} — ${headline}${headlineSuffix ?? ""}` : title
      }
    >
      <Row style={{ justifyContent: "space-between", alignItems: "flex-start" }}>
        <Column gap={t.space(1)}>
          <Text variant="caption" color="muted">
            {title}
          </Text>
          {headline || delta ? (
            <Row gap={t.space(2)} style={{ alignItems: "baseline" }}>
              {headline ? (
                <Row gap={1} style={{ alignItems: "baseline" }}>
                  <Text variant="title" weight="semibold">
                    {headline}
                  </Text>
                  {headlineSuffix ? (
                    <Text variant="body" color="muted">
                      {headlineSuffix}
                    </Text>
                  ) : null}
                </Row>
              ) : null}
              {delta ? (
                <Text variant="label" color="primary">
                  {delta}
                </Text>
              ) : null}
            </Row>
          ) : null}
        </Column>
        {periods && periods.length > 0 ? (
          <Row gap={t.space(1)}>
            {periods.map((p, i) => (
              <Button
                key={p}
                variant={i === activePeriodIndex ? "default" : "ghost"}
                size="sm"
                onPress={() => onPeriodChange?.(i)}
                accessibilityLabel={p}
              >
                {p}
              </Button>
            ))}
          </Row>
        ) : null}
      </Row>

      <MotiView
        from={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ type: "timing", duration: t.duration("slower") || 1600 }}
        style={{ width: "100%" }}
      >
        <Svg viewBox={`0 0 ${W} ${H}`} width="100%" height={H}>
          <Defs>
            <LinearGradient id="perf-area-grad" x1="0" x2="0" y1="0" y2="1">
              <Stop offset="0%" stopColor={stroke} stopOpacity="0.25" />
              <Stop offset="100%" stopColor={stroke} stopOpacity="0" />
            </LinearGradient>
          </Defs>

          {[0.25, 0.5, 0.75].map((p) => (
            <Line
              key={p}
              x1={PAD_X}
              x2={W - PAD_X}
              y1={PAD_Y + (H - PAD_Y * 2) * p}
              y2={PAD_Y + (H - PAD_Y * 2) * p}
              stroke={t.color("border")}
              strokeOpacity={0.4}
            />
          ))}

          {area ? <Path d={area} fill="url(#perf-area-grad)" /> : null}
          {path ? (
            <Path
              d={path}
              fill="none"
              stroke={stroke}
              strokeWidth={2}
              strokeLinecap="round"
            />
          ) : null}
        </Svg>
      </MotiView>

      <Row style={{ justifyContent: "space-between", paddingHorizontal: t.space(6) }}>
        {labels.map((l) => (
          <Text key={l} variant="caption" color="muted">
            {l}
          </Text>
        ))}
      </Row>
    </Card>
  );
}
