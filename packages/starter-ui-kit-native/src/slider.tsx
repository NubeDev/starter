// `<Slider>` — single-thumb continuous slider.
//
// The web Radix slider supports multi-thumb (range) selection; on
// mobile that's a separate UX (two thumbs touching is fiddly), so we
// ship single-thumb here and document multi-thumb as YAGNI until a
// renderer asks for it. Prop names mirror the web equivalent.

import * as React from "react";
import { PanResponder, StyleSheet, View } from "react-native";

import { useTheme } from "./theme.js";

type AnyEvt = unknown;
type AnyGesture = { dx: number; moveX: number; x0: number };

export interface SliderProps {
  value?: number;
  defaultValue?: number;
  onValueChange?: (next: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  /** Required — VoiceOver/TalkBack announce the value via this label. */
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
}

export function Slider({
  value,
  defaultValue,
  onValueChange,
  min = 0,
  max = 100,
  step = 1,
  disabled = false,
  accessibilityLabel,
  accessibilityHint,
  testID,
}: SliderProps): React.ReactElement {
  const [internal, setInternal] = React.useState(defaultValue ?? min);
  const current = value ?? internal;
  const setValue = (n: number) => {
    const clamped = Math.max(min, Math.min(max, n));
    const stepped = step > 0 ? Math.round(clamped / step) * step : clamped;
    if (value === undefined) setInternal(stepped);
    onValueChange?.(stepped);
  };

  const [width, setWidth] = React.useState(0);
  const t = useTheme();

  const responder = React.useMemo(
    () =>
      // eslint-disable-next-line @typescript-eslint/no-unsafe-call
      PanResponder.create({
        onStartShouldSetPanResponder: () => !disabled,
        onMoveShouldSetPanResponder: () => !disabled,
        onPanResponderMove: (_e: AnyEvt, g: AnyGesture) => {
          if (!width) return;
          const ratio = Math.max(0, Math.min(1, (g.moveX - g.x0 + offsetX(current, min, max, width)) / width));
          setValue(min + (max - min) * ratio);
        },
      }),
    // Recreate when the closed-over signals change so the move handler
    // sees the latest `width` / `current` / disabled state.
    [width, current, min, max, disabled, value],
  );

  const pct = max === min ? 0 : (current - min) / (max - min);
  const styles = StyleSheet.create({
    root: { width: "100%", height: 32, justifyContent: "center", opacity: disabled ? 0.5 : 1 },
    track: { height: 8, borderRadius: 4, backgroundColor: t.color("input") },
    fill: {
      position: "absolute",
      left: 0,
      top: 12,
      height: 8,
      borderRadius: 4,
      backgroundColor: t.color("primary"),
    },
    thumb: {
      position: "absolute",
      top: 6,
      width: 20,
      height: 20,
      borderRadius: 10,
      backgroundColor: t.color("background"),
      borderWidth: 1,
      borderColor: t.color("border"),
    },
  });
  return (
    <View
      accessible
      accessibilityRole="adjustable"
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      accessibilityValue={{ min, max, now: current }}
      accessibilityState={{ disabled }}
      testID={testID}
      style={styles.root}
      onLayout={(e: unknown) => {
        const ev = e as { nativeEvent?: { layout?: { width?: number } } };
        const w = ev.nativeEvent?.layout?.width;
        if (typeof w === "number") setWidth(w);
      }}
      {...responder.panHandlers}
    >
      <View style={styles.track} />
      <View style={[styles.fill, { width: pct * width }]} />
      <View style={[styles.thumb, { left: Math.max(0, pct * width - 10) }]} />
    </View>
  );
}

function offsetX(current: number, min: number, max: number, width: number): number {
  if (max === min) return 0;
  return ((current - min) / (max - min)) * width;
}
