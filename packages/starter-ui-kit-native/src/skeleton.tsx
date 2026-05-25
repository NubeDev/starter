// `<Skeleton>` — pulsing rectangular placeholder. Web kit uses
// `animate-pulse`; on RN we drive opacity with `moti`. Marked
// `accessibilityElementsHidden`-equivalent: `accessibilityRole="none"`
// and `accessible={false}` so it does not announce loading skeletons
// to screen readers (the surrounding region should own that message).

import * as React from "react";
import { StyleSheet } from "react-native";
import { MotiView } from "moti";

import { useTheme } from "./theme.js";

export interface SkeletonProps {
  width?: number | string;
  height?: number | string;
  radius?: number;
  /** Override the default `accessible={false}` if a consumer wants
   * the skeleton announced (rare; usually you want it hidden). */
  accessibilityLabel?: string;
  testID?: string;
}

export function Skeleton({
  width = "100%",
  height = 16,
  radius,
  accessibilityLabel,
  testID,
}: SkeletonProps): React.ReactElement {
  const t = useTheme();
  const styles = StyleSheet.create({
    box: {
      width: width as unknown as number,
      height: height as unknown as number,
      borderRadius: radius ?? t.radius("2xl"),
      backgroundColor: t.color("muted"),
    },
  });
  return (
    <MotiView
      accessible={Boolean(accessibilityLabel)}
      accessibilityRole={accessibilityLabel ? "progressbar" : "none"}
      accessibilityLabel={accessibilityLabel}
      testID={testID}
      from={{ opacity: 0.5 }}
      animate={{ opacity: 1 }}
      transition={{
        type: "timing",
        duration: t.duration("slower") || 800,
        loop: true,
      }}
      style={styles.box}
    />
  );
}
