// `<Button>` — mirrors `starter-ui-kit/src/components/ui/button.tsx`.
//
// Prop API parity:
//   web   : <Button variant="outline" size="sm" onClick={…}>
//   native: <Button variant="outline" size="sm" onPress={…}>
//
// Acceptance: every button ships `accessibilityRole="button"` plus a
// resolvable label (explicit `accessibilityLabel` wins; otherwise the
// `string` child is used). Reviewers may block a PR that breaks this.

import * as React from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { MotiView } from "moti";

import { useTheme } from "./theme.js";

export type ButtonVariant =
  | "default"
  | "outline"
  | "secondary"
  | "ghost"
  | "destructive"
  | "link";
export type ButtonSize = "xs" | "sm" | "default" | "lg" | "icon";

export interface ButtonProps {
  variant?: ButtonVariant;
  size?: ButtonSize;
  disabled?: boolean;
  onPress?: () => void;
  onLongPress?: () => void;
  /** Required when the button has no string child — accessibility gate. */
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
  children?: React.ReactNode;
}

function resolveLabel(
  explicit: string | undefined,
  children: React.ReactNode,
): string | undefined {
  if (explicit) return explicit;
  if (typeof children === "string") return children;
  if (typeof children === "number") return String(children);
  return undefined;
}

export function Button(props: ButtonProps): React.ReactElement {
  const {
    variant = "default",
    size = "default",
    disabled = false,
    onPress,
    onLongPress,
    accessibilityLabel,
    accessibilityHint,
    testID,
    children,
  } = props;

  const t = useTheme();
  const role = t.role(variant === "destructive" ? "danger" : "primary");

  const isGhostLike = variant === "ghost" || variant === "link";
  const isOutline = variant === "outline";
  const height = size === "xs" ? 24 : size === "sm" ? 32 : size === "lg" ? 40 : 36;
  const px = size === "icon" ? 0 : t.space(3);

  const background = isGhostLike
    ? "transparent"
    : isOutline
      ? t.color("background")
      : variant === "secondary"
        ? t.color("secondary")
        : variant === "destructive"
          ? role.background
          : t.color("primary");
  const foreground = isGhostLike
    ? variant === "link"
      ? t.color("primary")
      : t.color("foreground")
    : isOutline
      ? t.color("foreground")
      : variant === "secondary"
        ? t.color("secondary-foreground")
        : variant === "destructive"
          ? role.foreground
          : t.color("primary-foreground");

  const styles = StyleSheet.create({
    base: {
      flexDirection: "row",
      alignItems: "center",
      justifyContent: "center",
      height,
      borderRadius: t.radius("4xl"),
      paddingHorizontal: px,
      backgroundColor: background,
      borderWidth: isOutline ? 1 : 0,
      borderColor: isOutline ? t.color("border") : "transparent",
      opacity: disabled ? 0.5 : 1,
    },
    label: {
      color: foreground,
      fontSize: t.fontSize("sm"),
      fontWeight: String(t.fontWeight("medium")) as "500",
      textDecorationLine: variant === "link" ? "underline" : "none",
    },
  });

  const label = resolveLabel(accessibilityLabel, children);

  return (
    <MotiView
      from={{ opacity: 0.85 }}
      animate={{ opacity: disabled ? 0.5 : 1 }}
      transition={{ type: "timing", duration: t.duration("fast") }}
    >
      <Pressable
        accessible
        accessibilityRole="button"
        accessibilityLabel={label}
        accessibilityHint={accessibilityHint}
        accessibilityState={{ disabled }}
        disabled={disabled}
        onPress={onPress}
        onLongPress={onLongPress}
        testID={testID}
        style={styles.base}
      >
        {typeof children === "string" || typeof children === "number" ? (
          <Text style={styles.label}>{children}</Text>
        ) : (
          <View style={{ flexDirection: "row", alignItems: "center" }}>{children}</View>
        )}
      </Pressable>
    </MotiView>
  );
}
