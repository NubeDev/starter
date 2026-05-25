// `<Dialog>` — centred modal. Mirrors the web composition
// (`Dialog`, `DialogTrigger`, `DialogContent`, `DialogHeader`,
// `DialogTitle`, `DialogDescription`, `DialogFooter`, `DialogClose`).

import * as React from "react";
import { Modal, Pressable, StyleSheet, Text, View } from "react-native";
import { MotiView } from "moti";

import { useTheme } from "./theme.js";

interface DialogCtx {
  open: boolean;
  setOpen: (b: boolean) => void;
}
const Ctx = React.createContext<DialogCtx | null>(null);
function useDialog(): DialogCtx {
  const c = React.useContext(Ctx);
  if (!c) throw new Error("Dialog subcomponent used outside <Dialog>");
  return c;
}

export interface DialogProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (b: boolean) => void;
  children?: React.ReactNode;
}

export function Dialog({
  open,
  defaultOpen,
  onOpenChange,
  children,
}: DialogProps): React.ReactElement {
  const [internal, setInternal] = React.useState(defaultOpen ?? false);
  const current = open ?? internal;
  const setOpen = (b: boolean) => {
    if (open === undefined) setInternal(b);
    onOpenChange?.(b);
  };
  return <Ctx.Provider value={{ open: current, setOpen }}>{children}</Ctx.Provider>;
}

export function DialogTrigger({
  accessibilityLabel,
  children,
}: {
  accessibilityLabel?: string;
  children?: React.ReactNode;
}) {
  const d = useDialog();
  return (
    <Pressable
      accessible
      accessibilityRole="button"
      accessibilityLabel={
        accessibilityLabel ??
        (typeof children === "string" ? children : "Open dialog")
      }
      onPress={() => d.setOpen(true)}
    >
      {children}
    </Pressable>
  );
}

export function DialogContent({
  accessibilityLabel,
  children,
}: {
  accessibilityLabel?: string;
  children?: React.ReactNode;
}) {
  const t = useTheme();
  const d = useDialog();
  return (
    <Modal
      visible={d.open}
      transparent
      animationType="fade"
      onRequestClose={() => d.setOpen(false)}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Close dialog"
        onPress={() => d.setOpen(false)}
        style={[
          StyleSheet.absoluteFillObject as Record<string, unknown>,
          { backgroundColor: "rgba(0,0,0,0.4)" },
        ]}
      />
      <View
        accessible
        accessibilityRole="alert"
        accessibilityLabel={accessibilityLabel ?? "Dialog"}
        style={{
          flex: 1,
          alignItems: "center",
          justifyContent: "center",
          padding: t.space(6),
        }}
      >
        <MotiView
          from={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ type: "timing", duration: t.duration("normal") }}
          style={{
            width: "100%",
            maxWidth: 480,
            backgroundColor: t.color("popover"),
            borderRadius: t.radius("4xl"),
            borderWidth: 1,
            borderColor: t.color("border"),
            padding: t.space(6),
            gap: t.space(4),
          }}
        >
          {children}
        </MotiView>
      </View>
    </Modal>
  );
}

export function DialogHeader({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return <View style={{ gap: t.space(2) }}>{children}</View>;
}
export function DialogTitle({ children }: { children?: React.ReactNode }) {
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
export function DialogDescription({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return (
    <Text style={{ color: t.color("muted-foreground"), fontSize: t.fontSize("sm") }}>
      {children}
    </Text>
  );
}
export function DialogFooter({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  return (
    <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: t.space(2) }}>
      {children}
    </View>
  );
}
export function DialogClose({
  accessibilityLabel = "Close",
  children,
}: {
  accessibilityLabel?: string;
  children?: React.ReactNode;
}) {
  const d = useDialog();
  return (
    <Pressable
      accessible
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel}
      onPress={() => d.setOpen(false)}
    >
      {children}
    </Pressable>
  );
}
