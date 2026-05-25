// `<Tooltip>` — long-press hint. RN has no native hover; the canonical
// mobile gesture is long-press, so we expose a `Pressable` wrapper
// that toggles a bubble on `onLongPress`. The web kit's
// `TooltipProvider` / `TooltipTrigger` / `TooltipContent` composition
// is mirrored so call-sites are diff-clean across platforms.

import * as React from "react";
import { Pressable, Text, View } from "react-native";
import { MotiView } from "moti";

import { useTheme } from "./theme.js";

export interface TooltipProviderProps {
  /** Web parity — accepted but no-op on mobile (long-press has no delay knob). */
  delayDuration?: number;
  children?: React.ReactNode;
}
export function TooltipProvider({ children }: TooltipProviderProps): React.ReactElement {
  return <>{children}</>;
}

interface TooltipCtx {
  open: boolean;
  setOpen: (b: boolean) => void;
  label: string | undefined;
  setLabel: (s: string | undefined) => void;
}
const Ctx = React.createContext<TooltipCtx | null>(null);
function useTip(): TooltipCtx {
  const c = React.useContext(Ctx);
  if (!c) throw new Error("Tooltip subcomponent used outside <Tooltip>");
  return c;
}

export interface TooltipProps {
  children?: React.ReactNode;
}
export function Tooltip({ children }: TooltipProps): React.ReactElement {
  const [open, setOpen] = React.useState(false);
  const [label, setLabel] = React.useState<string | undefined>(undefined);
  return <Ctx.Provider value={{ open, setOpen, label, setLabel }}>{children}</Ctx.Provider>;
}

export interface TooltipTriggerProps {
  accessibilityLabel?: string;
  accessibilityHint?: string;
  children?: React.ReactNode;
}
export function TooltipTrigger({
  accessibilityLabel,
  accessibilityHint,
  children,
}: TooltipTriggerProps): React.ReactElement {
  const tip = useTip();
  return (
    <Pressable
      accessible
      accessibilityRole="button"
      accessibilityLabel={
        accessibilityLabel ??
        (typeof children === "string" ? children : "Show tooltip")
      }
      accessibilityHint={accessibilityHint ?? "Long-press to see tooltip"}
      onLongPress={() => tip.setOpen(true)}
      onPress={() => tip.setOpen(false)}
    >
      {children}
    </Pressable>
  );
}

export interface TooltipContentProps {
  /** The content text; also surfaces as the bubble's a11y label. */
  children: string;
  testID?: string;
}
export function TooltipContent({
  children,
  testID,
}: TooltipContentProps): React.ReactElement | null {
  const t = useTheme();
  const tip = useTip();
  React.useEffect(() => {
    tip.setLabel(children);
  }, [children, tip]);
  if (!tip.open) return null;
  return (
    <MotiView
      from={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ type: "timing", duration: t.duration("fast") }}
      style={{
        position: "absolute",
        bottom: -32,
        alignSelf: "center",
        backgroundColor: t.color("popover"),
        borderColor: t.color("border"),
        borderWidth: 1,
        borderRadius: t.radius("md"),
        paddingHorizontal: t.space(2),
        paddingVertical: t.space(1),
      }}
    >
      <View accessible accessibilityRole="text" accessibilityLabel={children} testID={testID}>
        <Text style={{ color: t.color("popover-foreground"), fontSize: t.fontSize("xs") }}>
          {children}
        </Text>
      </View>
    </MotiView>
  );
}
