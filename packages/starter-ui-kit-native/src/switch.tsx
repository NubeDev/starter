// `<Switch>` — controlled boolean. Mirrors the web `Switch`. Uses
// `Pressable` directly rather than RN's built-in `Switch` because
// the built-in is unstyled and we want the visual to match the kit.

import * as React from "react";
import { Pressable, StyleSheet, View } from "react-native";
import { MotiView } from "moti";

import { useTheme } from "./theme.js";

export interface SwitchProps {
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (next: boolean) => void;
  disabled?: boolean;
  size?: "sm" | "default";
  /** Required for non-decorative switches — VoiceOver announces it. */
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
}

export function Switch({
  checked,
  defaultChecked,
  onCheckedChange,
  disabled = false,
  size = "default",
  accessibilityLabel,
  accessibilityHint,
  testID,
}: SwitchProps): React.ReactElement {
  const [internal, setInternal] = React.useState(defaultChecked ?? false);
  const value = checked ?? internal;
  const toggle = () => {
    if (disabled) return;
    const next = !value;
    if (checked === undefined) setInternal(next);
    onCheckedChange?.(next);
  };
  const t = useTheme();
  const trackW = size === "sm" ? 28 : 44;
  const trackH = size === "sm" ? 16 : 20;
  const thumb = size === "sm" ? 12 : 16;
  const styles = StyleSheet.create({
    track: {
      width: trackW,
      height: trackH,
      borderRadius: trackH / 2,
      backgroundColor: value ? t.color("primary") : t.color("input"),
      padding: 2,
      opacity: disabled ? 0.5 : 1,
      justifyContent: "center",
    },
    thumb: {
      width: thumb,
      height: thumb,
      borderRadius: thumb / 2,
      backgroundColor: t.color("background"),
    },
  });
  return (
    <Pressable
      accessible
      accessibilityRole="switch"
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      accessibilityState={{ checked: value, disabled }}
      disabled={disabled}
      onPress={toggle}
      testID={testID}
    >
      <View style={styles.track}>
        <MotiView
          animate={{ translateX: value ? trackW - thumb - 4 : 0 }}
          transition={{ type: "timing", duration: t.duration("fast") }}
          style={styles.thumb}
        />
      </View>
    </Pressable>
  );
}
