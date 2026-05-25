// `<Card>` + subcomponents — mirrors the seven-piece web `Card`
// composition (`Card`, `CardHeader`, `CardTitle`, `CardDescription`,
// `CardAction`, `CardContent`, `CardFooter`). The web kit's
// "data-size" prop becomes a strongly-typed `size` prop here.
//
// Accessibility: the outer `Card` uses `accessibilityRole="summary"`
// by default so screen readers announce the group cleanly; callers
// can override.

import * as React from "react";
import { StyleSheet, Text, View } from "react-native";

import { useTheme } from "./theme.js";

export interface CardProps {
  size?: "default" | "sm";
  accessibilityLabel?: string;
  accessibilityHint?: string;
  accessibilityRole?: string;
  testID?: string;
  children?: React.ReactNode;
}

export function Card({
  size = "default",
  accessibilityLabel,
  accessibilityHint,
  accessibilityRole = "summary",
  testID,
  children,
}: CardProps): React.ReactElement {
  const t = useTheme();
  const gap = size === "sm" ? t.space(4) : t.space(6);
  const py = size === "sm" ? t.space(4) : t.space(6);
  const styles = StyleSheet.create({
    card: {
      flexDirection: "column",
      gap,
      paddingVertical: py,
      backgroundColor: t.color("card"),
      borderRadius: t.radius("4xl"),
      borderWidth: 1,
      borderColor: t.color("border"),
      overflow: "hidden",
    },
  });
  return (
    <View
      accessible
      accessibilityRole={accessibilityRole}
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      testID={testID}
      style={styles.card}
    >
      {children}
    </View>
  );
}

function pad(t: ReturnType<typeof useTheme>) {
  return { paddingHorizontal: t.space(6) };
}

export function CardHeader({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return <View style={[pad(t), { gap: t.space(2) }]}>{children}</View>;
}

export function CardTitle({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return (
    <Text
      accessibilityRole="header"
      style={{
        color: t.color("card-foreground"),
        fontSize: t.fontSize("base"),
        fontWeight: String(t.fontWeight("medium")) as "500",
      }}
    >
      {children}
    </Text>
  );
}

export function CardDescription({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return (
    <Text
      style={{
        color: t.color("muted-foreground"),
        fontSize: t.fontSize("sm"),
      }}
    >
      {children}
    </Text>
  );
}

export function CardAction({ children }: { children?: React.ReactNode }) {
  return (
    <View style={{ alignSelf: "flex-end" }}>{children}</View>
  );
}

export function CardContent({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return <View style={pad(t)}>{children}</View>;
}

export function CardFooter({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return (
    <View style={[pad(t), { flexDirection: "row", alignItems: "center" }]}>
      {children}
    </View>
  );
}
