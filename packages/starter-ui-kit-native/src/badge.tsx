// `<Badge>` — mirrors `starter-ui-kit/src/components/ui/badge.tsx`.
// Single-element pill; a11y role defaults to "text" because most
// badges are decorative — callers wanting an interactive badge swap
// in `<Button variant="ghost" size="xs">` instead.

import * as React from "react";
import { StyleSheet, Text, View } from "react-native";

import { useTheme } from "./theme.js";

export type BadgeVariant =
  | "default"
  | "secondary"
  | "destructive"
  | "outline"
  | "ghost"
  | "link";

export interface BadgeProps {
  variant?: BadgeVariant;
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
  children?: React.ReactNode;
}

export function Badge({
  variant = "default",
  accessibilityLabel,
  accessibilityHint,
  testID,
  children,
}: BadgeProps): React.ReactElement {
  const t = useTheme();
  const role =
    variant === "destructive" ? t.role("danger") : t.role(variantToRole(variant));
  const bg =
    variant === "outline" || variant === "ghost" || variant === "link"
      ? "transparent"
      : role.background;
  const fg =
    variant === "link"
      ? t.color("primary")
      : variant === "outline" || variant === "ghost"
        ? t.color("foreground")
        : role.foreground;

  const styles = StyleSheet.create({
    pill: {
      paddingHorizontal: t.space(2),
      paddingVertical: t.space(0.5),
      borderRadius: t.radius("3xl"),
      backgroundColor: bg,
      borderWidth: variant === "outline" ? 1 : 0,
      borderColor: variant === "outline" ? t.color("border") : "transparent",
      alignSelf: "flex-start",
    },
    txt: {
      color: fg,
      fontSize: t.fontSize("xs"),
      fontWeight: String(t.fontWeight("medium")) as "500",
    },
  });

  const label =
    accessibilityLabel ?? (typeof children === "string" ? children : undefined);

  return (
    <View
      accessible
      accessibilityRole="text"
      accessibilityLabel={label}
      accessibilityHint={accessibilityHint}
      testID={testID}
      style={styles.pill}
    >
      <Text style={styles.txt}>{children}</Text>
    </View>
  );
}

function variantToRole(v: BadgeVariant) {
  switch (v) {
    case "secondary":
      return "secondary" as const;
    case "destructive":
      return "danger" as const;
    default:
      return "primary" as const;
  }
}
