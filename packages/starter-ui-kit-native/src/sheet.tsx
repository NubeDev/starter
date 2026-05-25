// `<Sheet>` — native bottom sheet, replaces Radix Sheet on web.
// Mirrors the web composition: `Sheet`, `SheetTrigger`, `SheetContent`,
// `SheetHeader`, `SheetTitle`, `SheetDescription`, `SheetClose`.

import * as React from "react";
import { Modal, Pressable, StyleSheet, Text, View } from "react-native";
import { MotiView } from "moti";

import { useTheme } from "./theme.js";

interface SheetCtx {
  open: boolean;
  setOpen: (b: boolean) => void;
  side: "bottom" | "top" | "left" | "right";
}
const Ctx = React.createContext<SheetCtx | null>(null);
function useSheet(): SheetCtx {
  const c = React.useContext(Ctx);
  if (!c) throw new Error("Sheet subcomponent used outside <Sheet>");
  return c;
}

export interface SheetProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (b: boolean) => void;
  side?: "bottom" | "top" | "left" | "right";
  children?: React.ReactNode;
}

export function Sheet({
  open,
  defaultOpen,
  onOpenChange,
  side = "bottom",
  children,
}: SheetProps): React.ReactElement {
  const [internal, setInternal] = React.useState(defaultOpen ?? false);
  const current = open ?? internal;
  const setOpen = (b: boolean) => {
    if (open === undefined) setInternal(b);
    onOpenChange?.(b);
  };
  return <Ctx.Provider value={{ open: current, setOpen, side }}>{children}</Ctx.Provider>;
}

export interface SheetTriggerProps {
  accessibilityLabel?: string;
  accessibilityHint?: string;
  children?: React.ReactNode;
}

export function SheetTrigger({
  accessibilityLabel,
  accessibilityHint,
  children,
}: SheetTriggerProps): React.ReactElement {
  const s = useSheet();
  return (
    <Pressable
      accessible
      accessibilityRole="button"
      accessibilityLabel={
        accessibilityLabel ??
        (typeof children === "string" ? children : "Open sheet")
      }
      accessibilityHint={accessibilityHint}
      onPress={() => s.setOpen(true)}
    >
      {children}
    </Pressable>
  );
}

export function SheetContent({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  const s = useSheet();
  const isBottom = s.side === "bottom";
  return (
    <Modal
      visible={s.open}
      transparent
      animationType="slide"
      onRequestClose={() => s.setOpen(false)}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Close sheet"
        onPress={() => s.setOpen(false)}
        style={[
          { position: "absolute", top: 0, left: 0, right: 0, bottom: 0 } as Record<string, unknown>,
          { backgroundColor: "rgba(0,0,0,0.4)" },
        ]}
      />
      <MotiView
        from={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ type: "timing", duration: t.duration("normal") }}
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          [isBottom ? "bottom" : "top"]: 0,
          backgroundColor: t.color("popover"),
          borderTopLeftRadius: t.radius("4xl"),
          borderTopRightRadius: t.radius("4xl"),
          padding: t.space(6),
          gap: t.space(4),
        }}
      >
        <View
          accessible
          accessibilityRole="none"
          accessibilityLabel="Sheet content"
        >
          {children}
        </View>
      </MotiView>
    </Modal>
  );
}

export function SheetHeader({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return <View style={{ gap: t.space(2) }}>{children}</View>;
}
export function SheetTitle({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return (
    <Text
      accessibilityRole="header"
      style={{
        color: t.color("popover-foreground"),
        fontSize: t.fontSize("lg"),
        fontWeight: String(t.fontWeight("semibold")) as "600",
      }}
    >
      {children}
    </Text>
  );
}
export function SheetDescription({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return (
    <Text style={{ color: t.color("muted-foreground"), fontSize: t.fontSize("sm") }}>
      {children}
    </Text>
  );
}
export function SheetClose({
  accessibilityLabel = "Close",
  children,
}: {
  accessibilityLabel?: string;
  children?: React.ReactNode;
}) {
  const s = useSheet();
  return (
    <Pressable
      accessible
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel}
      onPress={() => s.setOpen(false)}
    >
      {children}
    </Pressable>
  );
}
