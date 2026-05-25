// `<Select>` — controlled dropdown. The web kit composes
// `Select` / `SelectTrigger` / `SelectContent` / `SelectItem`; we
// mirror the same surface. On mobile the "content" is rendered in a
// modal sheet because there's no popover layer on RN.
//
// Single-select only — multi-select is YAGNI until a renderer asks.

import * as React from "react";
import { Modal, Pressable, ScrollView, StyleSheet, Text, View } from "react-native";

import { useTheme } from "./theme.js";

interface SelectCtx {
  value: string | undefined;
  setValue: (v: string) => void;
  open: boolean;
  setOpen: (b: boolean) => void;
  label: string | undefined;
  setLabel: (s: string) => void;
}
const Ctx = React.createContext<SelectCtx | null>(null);
function useSel(): SelectCtx {
  const c = React.useContext(Ctx);
  if (!c) throw new Error("Select subcomponent used outside <Select>");
  return c;
}

export interface SelectProps {
  value?: string;
  defaultValue?: string;
  onValueChange?: (v: string) => void;
  children?: React.ReactNode;
}

export function Select({
  value,
  defaultValue,
  onValueChange,
  children,
}: SelectProps): React.ReactElement {
  const [internal, setInternal] = React.useState(defaultValue);
  const [open, setOpen] = React.useState(false);
  const [label, setLabel] = React.useState<string | undefined>(undefined);
  const current = value ?? internal;
  const setValue = (v: string) => {
    if (value === undefined) setInternal(v);
    onValueChange?.(v);
    setOpen(false);
  };
  return (
    <Ctx.Provider
      value={{ value: current, setValue, open, setOpen, label, setLabel }}
    >
      {children}
    </Ctx.Provider>
  );
}

export interface SelectTriggerProps {
  placeholder?: string;
  disabled?: boolean;
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
}

export function SelectTrigger({
  placeholder,
  disabled = false,
  accessibilityLabel,
  accessibilityHint,
  testID,
}: SelectTriggerProps): React.ReactElement {
  const t = useTheme();
  const sel = useSel();
  return (
    <Pressable
      accessible
      accessibilityRole="combobox"
      accessibilityLabel={accessibilityLabel ?? sel.label ?? placeholder}
      accessibilityHint={accessibilityHint}
      accessibilityState={{ expanded: sel.open, disabled }}
      disabled={disabled}
      testID={testID}
      onPress={() => sel.setOpen(true)}
      style={{
        minHeight: 36,
        paddingHorizontal: t.space(3),
        borderRadius: t.radius("3xl"),
        borderWidth: 1,
        borderColor: t.color("border"),
        backgroundColor: t.color("input"),
        flexDirection: "row",
        alignItems: "center",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      <Text
        style={{
          color: sel.label ? t.color("foreground") : t.color("muted-foreground"),
          fontSize: t.fontSize("sm"),
        }}
      >
        {sel.label ?? placeholder ?? ""}
      </Text>
    </Pressable>
  );
}

export function SelectContent({ children }: { children?: React.ReactNode }) {
  const t = useTheme();
  const sel = useSel();
  return (
    <Modal
      visible={sel.open}
      transparent
      animationType="fade"
      onRequestClose={() => sel.setOpen(false)}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Close menu"
        onPress={() => sel.setOpen(false)}
        style={StyleSheet.absoluteFillObject as Record<string, unknown>}
      />
      <View
        style={{
          marginTop: 120,
          marginHorizontal: t.space(6),
          backgroundColor: t.color("popover"),
          borderRadius: t.radius("3xl"),
          borderWidth: 1,
          borderColor: t.color("border"),
          padding: t.space(2),
          maxHeight: 360,
        }}
      >
        <ScrollView accessibilityRole="menu">{children}</ScrollView>
      </View>
    </Modal>
  );
}

export interface SelectItemProps {
  value: string;
  label?: string;
  disabled?: boolean;
  children?: React.ReactNode;
}

export function SelectItem({
  value,
  label,
  disabled = false,
  children,
}: SelectItemProps): React.ReactElement {
  const t = useTheme();
  const sel = useSel();
  const selected = sel.value === value;
  const display = label ?? (typeof children === "string" ? children : value);
  return (
    <Pressable
      accessible
      accessibilityRole="menuitem"
      accessibilityLabel={display}
      accessibilityState={{ selected, disabled }}
      disabled={disabled}
      onPress={() => {
        sel.setLabel(display);
        sel.setValue(value);
      }}
      style={{
        paddingHorizontal: t.space(3),
        paddingVertical: t.space(2),
        borderRadius: t.radius("md"),
        backgroundColor: selected ? t.color("muted") : "transparent",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      <Text style={{ color: t.color("popover-foreground"), fontSize: t.fontSize("sm") }}>
        {children ?? display}
      </Text>
    </Pressable>
  );
}
