// `<RadialProgress>` — RN port of
// `starter-ui-dashboard/src/radial-progress.tsx`. Prop API mirrors
// the web version one-to-one.
//
// SVG circle stroke is sized by `size` + `stroke`; the filled arc is
// animated by setting the `MotiView` opacity (the stroke geometry is
// deterministic and computed once on render).

import * as React from "react";
import { MotiView } from "moti";
import { Circle, Svg } from "react-native-svg";

import { Box, Card, Column, Row, Text, useTheme } from "@nube/starter-ui-kit-native";

export interface RadialProgressProps {
  /** Progress value, 0–100. Clamped at render time. */
  value: number;
  /** Top-of-card label (e.g. "Battery"). Already localized. */
  label: string;
  /** Optional caption under the percentage (e.g. "12h remaining"). */
  subLabel?: string;
  /** Diameter in pixels. Default 180. */
  size?: number;
  /** Stroke width of the ring in pixels. Default 10. */
  stroke?: number;
  /** Stroke colour for the filled arc. Defaults to the theme `primary`
   * token. Pass any CSS colour string. */
  accent?: string;
  /** Reserved for parity with the web component. Ignored on RN. */
  className?: string;
}

export function RadialProgress({
  value,
  label,
  subLabel,
  size = 180,
  stroke = 10,
  accent,
}: RadialProgressProps): React.ReactElement {
  const t = useTheme();
  const ringColor = accent ?? t.color("primary");
  const clamped = Math.max(0, Math.min(100, value));
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const offset = c - (clamped / 100) * c;
  const cx = size / 2;
  const cy = size / 2;

  return (
    <Card
      accessibilityRole="progressbar"
      accessibilityLabel={`${label}: ${clamped}%${subLabel ? ` — ${subLabel}` : ""}`}
    >
      <Text variant="caption" color="muted">
        {label}
      </Text>
      <Row style={{ justifyContent: "center", alignItems: "center", marginTop: t.space(4) }}>
        <Box style={{ position: "relative", width: size, height: size }}>
          <Svg
            width={size}
            height={size}
            style={{ transform: [{ rotate: "-90deg" }] }}
          >
            <Circle
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke={t.color("muted")}
              strokeWidth={stroke}
            />
            <Circle
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke={ringColor}
              strokeWidth={stroke}
              strokeLinecap="round"
              strokeDasharray={c}
              strokeDashoffset={offset}
            />
          </Svg>
          <MotiView
            from={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ type: "timing", duration: t.duration("slower") || 1600 }}
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <Column style={{ alignItems: "center" }}>
              <Row gap={2} style={{ alignItems: "baseline" }}>
                <Text variant="title" weight="semibold">
                  {String(clamped)}
                </Text>
                <Text variant="subtitle" color="muted">
                  %
                </Text>
              </Row>
              {subLabel ? (
                <Text variant="caption" color="muted">
                  {subLabel}
                </Text>
              ) : null}
            </Column>
          </MotiView>
        </Box>
      </Row>
    </Card>
  );
}
