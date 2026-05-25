// Mock `@nube/starter-ui-kit-native` for renderer unit tests.
//
// Each export is a thin host element whose tag name encodes the kit
// component (`kn-button`, `kn-card`, …) and whose data-* attributes
// echo the props. This gives the test the two things the renderer
// contract promises:
//
//   1. renderers depend ONLY on this surface — if a renderer reaches
//      for something this mock doesn't export, the import errors.
//   2. props (and a11y wiring) pass through verbatim, so tests can
//      assert on them.
//
// `useTheme()` returns a deterministic token bag — keeps snapshot
// tests stable.

import * as React from "react";

type AnyProps = Record<string, unknown> & { children?: React.ReactNode };

function el(tag: string) {
  return React.forwardRef<unknown, AnyProps>(function KitEl(props, ref) {
    const { children, style, ...rest } = props;
    const echoed: Record<string, unknown> = { "data-kit": tag, ref };
    for (const [k, v] of Object.entries(rest)) {
      if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
        echoed[`data-${k.toLowerCase()}`] = String(v);
      }
    }
    if (style !== undefined) echoed["data-style"] = JSON.stringify(style);
    return React.createElement(`kn-${tag}`, echoed, children as React.ReactNode);
  });
}

// Layout
export const Box = el("box");
export const Row = el("row");
export const Column = el("column");
export const ScrollArea = el("scroll-area");
export const Text = el("text");
export const Divider = el("divider");

// Primitives
export const Button = el("button");
export const Card = el("card");
export const CardHeader = el("card-header");
export const CardTitle = el("card-title");
export const CardDescription = el("card-description");
export const CardAction = el("card-action");
export const CardContent = el("card-content");
export const CardFooter = el("card-footer");
export const Input = el("input");

export const Tabs = el("tabs");
export const TabsList = el("tabs-list");
export const TabsTrigger = el("tabs-trigger");
export const TabsContent = el("tabs-content");

export const Badge = el("badge");
export const Switch = el("switch");
export const Slider = el("slider");

export const Select = el("select");
export const SelectTrigger = el("select-trigger");
export const SelectContent = el("select-content");
export const SelectItem = el("select-item");

export const Sheet = el("sheet");
export const SheetTrigger = el("sheet-trigger");
export const SheetContent = el("sheet-content");
export const SheetHeader = el("sheet-header");
export const SheetTitle = el("sheet-title");
export const SheetDescription = el("sheet-description");
export const SheetClose = el("sheet-close");

export const Dialog = el("dialog");
export const DialogTrigger = el("dialog-trigger");
export const DialogContent = el("dialog-content");
export const DialogHeader = el("dialog-header");
export const DialogTitle = el("dialog-title");
export const DialogDescription = el("dialog-description");
export const DialogFooter = el("dialog-footer");
export const DialogClose = el("dialog-close");

export const Spinner = el("spinner");
export const Skeleton = el("skeleton");

export const Tooltip = el("tooltip");
export const TooltipProvider = el("tooltip-provider");
export const TooltipTrigger = el("tooltip-trigger");
export const TooltipContent = el("tooltip-content");

export function useTheme() {
  return {
    mode: "light" as const,
    paletteId: "default",
    colors: {} as Record<string, string>,
    color: (_k: string) => "#000",
    role: (_r: string) => ({ background: "#fff", foreground: "#000", border: "#ccc" }),
    space: (n: number | string) => (typeof n === "number" ? n * 4 : 8),
    radius: (_s: string) => 8,
    fontSize: (_s: string) => 14,
    fontWeight: (_w: string) => 400,
    duration: (_k: string) => 150,
    easing: (_k: string) => [0.4, 0.0, 0.2, 1] as const,
    preferences: {
      mode: "light" as const,
      paletteId: "default",
      density: "comfortable" as const,
      fontSize: "md" as const,
      motion: "full" as const,
    },
  };
}
