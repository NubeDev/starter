// Layout primitives — thin RN wrappers exposed from the kit so
// downstream renderer packages (e.g. `@nube/starter-ui-sdui-native`)
// never import `react-native` directly. Keeping these in the kit
// makes the "renderers depend on kit only" rule enforceable and
// keeps the testable swap-in-a-mock-kit story honest.
//
// These are NOT a substitute for the styled primitives (`Card`,
// `Button`, …) — those still carry the design language. `Box`, `Row`,
// `Column`, `Text`, `ScrollArea` are the raw containers for SDUI
// `page`, `row`, `col`, `grid`, `divider` semantics.

import * as React from "react";
import {
  ScrollView as RNScrollView,
  StyleSheet,
  Text as RNText,
  View,
} from "react-native";

import { useTheme } from "./theme.js";

type Style = Record<string, unknown>;

export interface BoxProps {
  direction?: "row" | "column";
  gap?: number;
  padding?: number;
  flex?: number;
  wrap?: boolean;
  style?: Style;
  accessibilityRole?: string;
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
  children?: React.ReactNode;
}

export function Box(props: BoxProps): React.ReactElement {
  const {
    direction = "column",
    gap,
    padding,
    flex,
    wrap,
    style,
    children,
    accessibilityRole,
    accessibilityLabel,
    accessibilityHint,
    testID,
  } = props;
  const computed = StyleSheet.flatten([
    {
      flexDirection: direction,
      ...(gap !== undefined ? { gap } : {}),
      ...(padding !== undefined ? { padding } : {}),
      ...(flex !== undefined ? { flex } : {}),
      ...(wrap ? { flexWrap: "wrap" as const } : {}),
    },
    style ?? {},
  ]);
  return (
    <View
      style={computed}
      accessibilityRole={accessibilityRole as never}
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      testID={testID}
    >
      {children}
    </View>
  );
}

export function Row(props: Omit<BoxProps, "direction">): React.ReactElement {
  return <Box {...props} direction="row" />;
}

export function Column(
  props: Omit<BoxProps, "direction">,
): React.ReactElement {
  return <Box {...props} direction="column" />;
}

export interface TextProps {
  variant?: "title" | "subtitle" | "body" | "label" | "caption";
  weight?: "regular" | "medium" | "semibold" | "bold";
  color?: "foreground" | "muted" | "primary" | "destructive";
  numberOfLines?: number;
  style?: Style;
  accessibilityRole?: string;
  accessibilityLabel?: string;
  testID?: string;
  children?: React.ReactNode;
}

const VARIANT_SIZE: Record<NonNullable<TextProps["variant"]>, "xs" | "sm" | "base" | "lg" | "2xl"> =
  {
    title: "2xl",
    subtitle: "lg",
    body: "base",
    label: "sm",
    caption: "xs",
  };

const WEIGHT_KEY: Record<NonNullable<TextProps["weight"]>, "regular" | "medium" | "semibold" | "bold"> =
  {
    regular: "regular",
    medium: "medium",
    semibold: "semibold",
    bold: "bold",
  };

const COLOR_TOKEN = {
  foreground: "foreground",
  muted: "muted-foreground",
  primary: "primary",
  destructive: "destructive",
} as const;

export function Text(props: TextProps): React.ReactElement {
  const {
    variant = "body",
    weight = "regular",
    color = "foreground",
    numberOfLines,
    style,
    children,
    accessibilityRole,
    accessibilityLabel,
    testID,
  } = props;
  const t = useTheme();
  const computed = StyleSheet.flatten([
    {
      fontSize: t.fontSize(VARIANT_SIZE[variant]),
      fontWeight: String(t.fontWeight(WEIGHT_KEY[weight])) as never,
      color: t.color(COLOR_TOKEN[color]),
    },
    style ?? {},
  ]);
  return (
    <RNText
      style={computed}
      numberOfLines={numberOfLines}
      accessibilityRole={accessibilityRole as never}
      accessibilityLabel={accessibilityLabel}
      testID={testID}
    >
      {children}
    </RNText>
  );
}

export interface ScrollAreaProps {
  horizontal?: boolean;
  style?: Style;
  contentStyle?: Style;
  testID?: string;
  children?: React.ReactNode;
}

export function ScrollArea(props: ScrollAreaProps): React.ReactElement {
  const { horizontal, style, contentStyle, testID, children } = props;
  return (
    <RNScrollView
      horizontal={horizontal}
      style={style as never}
      contentContainerStyle={contentStyle as never}
      testID={testID}
    >
      {children}
    </RNScrollView>
  );
}

export interface DividerProps {
  orientation?: "horizontal" | "vertical";
  style?: Style;
  testID?: string;
}

export function Divider(props: DividerProps): React.ReactElement {
  const { orientation = "horizontal", style, testID } = props;
  const t = useTheme();
  const base: Style =
    orientation === "vertical"
      ? { width: StyleSheet.hairlineWidth, alignSelf: "stretch", backgroundColor: t.color("border") }
      : { height: StyleSheet.hairlineWidth, alignSelf: "stretch", backgroundColor: t.color("border") };
  return (
    <View
      style={StyleSheet.flatten([base, style ?? {}])}
      accessibilityRole={"separator" as never}
      testID={testID}
    />
  );
}
