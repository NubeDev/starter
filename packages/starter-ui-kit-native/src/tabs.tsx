// `<Tabs>` — controlled or uncontrolled, mirrors the four-piece web
// composition (`Tabs`, `TabsList`, `TabsTrigger`, `TabsContent`). On
// web the underlying primitive is Radix; on mobile we own the state
// (a tiny context) because there's no equivalent unstyled RN library
// in our toolchain yet.

import * as React from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";

import { useTheme } from "./theme.js";

interface TabsCtx {
  value: string;
  setValue: (v: string) => void;
  orientation: "horizontal" | "vertical";
}
const Ctx = React.createContext<TabsCtx | null>(null);
function useTabs(): TabsCtx {
  const ctx = React.useContext(Ctx);
  if (!ctx) throw new Error("Tabs subcomponent used outside <Tabs>");
  return ctx;
}

export interface TabsProps {
  value?: string;
  defaultValue?: string;
  onValueChange?: (v: string) => void;
  orientation?: "horizontal" | "vertical";
  children?: React.ReactNode;
}

export function Tabs({
  value,
  defaultValue,
  onValueChange,
  orientation = "horizontal",
  children,
}: TabsProps): React.ReactElement {
  const [internal, setInternal] = React.useState(defaultValue ?? "");
  const current = value ?? internal;
  const setValue = React.useCallback(
    (v: string) => {
      if (value === undefined) setInternal(v);
      onValueChange?.(v);
    },
    [value, onValueChange],
  );
  const t = useTheme();
  return (
    <Ctx.Provider value={{ value: current, setValue, orientation }}>
      <View
        accessibilityRole="tablist"
        style={{
          gap: t.space(2),
          flexDirection: orientation === "vertical" ? "row" : "column",
        }}
      >
        {children}
      </View>
    </Ctx.Provider>
  );
}

export function TabsList({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  const { orientation } = useTabs();
  return (
    <View
      style={{
        flexDirection: orientation === "vertical" ? "column" : "row",
        backgroundColor: t.color("muted"),
        borderRadius: t.radius("4xl"),
        padding: t.space(1),
      }}
    >
      {children}
    </View>
  );
}

export interface TabsTriggerProps {
  value: string;
  disabled?: boolean;
  accessibilityLabel?: string;
  accessibilityHint?: string;
  children?: React.ReactNode;
}

export function TabsTrigger({
  value,
  disabled = false,
  accessibilityLabel,
  accessibilityHint,
  children,
}: TabsTriggerProps): React.ReactElement {
  const t = useTheme();
  const ctx = useTabs();
  const active = ctx.value === value;
  const styles = StyleSheet.create({
    btn: {
      paddingHorizontal: t.space(3),
      paddingVertical: t.space(1),
      borderRadius: t.radius("4xl"),
      backgroundColor: active ? t.color("background") : "transparent",
      opacity: disabled ? 0.5 : 1,
    },
    txt: {
      color: active ? t.color("foreground") : t.color("muted-foreground"),
      fontSize: t.fontSize("sm"),
      fontWeight: String(t.fontWeight("medium")) as "500",
    },
  });
  const label =
    accessibilityLabel ?? (typeof children === "string" ? children : value);
  return (
    <Pressable
      accessible
      accessibilityRole="tab"
      accessibilityLabel={label}
      accessibilityHint={accessibilityHint}
      accessibilityState={{ selected: active, disabled }}
      disabled={disabled}
      onPress={() => ctx.setValue(value)}
      style={styles.btn}
    >
      <Text style={styles.txt}>{children}</Text>
    </Pressable>
  );
}

export interface TabsContentProps {
  value: string;
  children?: React.ReactNode;
}

export function TabsContent({
  value,
  children,
}: TabsContentProps): React.ReactElement | null {
  const ctx = useTabs();
  if (ctx.value !== value) return null;
  return (
    <View accessibilityRole={"tabpanel" as never} style={{ flex: 1 }}>
      {children}
    </View>
  );
}
