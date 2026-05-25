// Mock `@nube/starter-ui-kit-native` for widget unit tests.
//
// Each export is a thin host element whose tag name encodes the kit
// component (`kn-card`, `kn-column`, …) and whose data-* attributes
// echo the props. This gives the test the two things the widget
// contract promises:
//
//   1. widgets depend ONLY on this surface — if a widget reaches
//      for something this mock doesn't export, import errors out;
//   2. props (and a11y wiring) pass through verbatim, so tests can
//      assert on them.
//
// `useTheme()` returns a deterministic token bag — keeps snapshots
// stable and lets widgets compute layout/style without RN.

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

// Primitives (only the ones widgets touch — anything else is a
// deliberate import failure, which is the test we want).
export const Card = el("card");
export const CardHeader = el("card-header");
export const CardTitle = el("card-title");
export const CardDescription = el("card-description");
export const CardContent = el("card-content");
export const CardFooter = el("card-footer");
export const Button = el("button");
export const Badge = el("badge");

export function useTheme() {
  return {
    mode: "light" as const,
    paletteId: "default",
    colors: {} as Record<string, string>,
    color: (k: string) => `var(--${k})`,
    role: (_r: string) => ({ background: "#fff", foreground: "#000", border: "#ccc" }),
    space: (n: number | string) => (typeof n === "number" ? n * 4 : 8),
    radius: (_s: string) => 24,
    fontSize: (s: string) =>
      ({ xs: 11, sm: 13, base: 14, lg: 18, "2xl": 24 } as Record<string, number>)[s] ?? 14,
    fontWeight: (w: string) =>
      ({ regular: 400, medium: 500, semibold: 600, bold: 700 } as Record<string, number>)[w] ?? 400,
    duration: (_k: string) => 150,
    easing: (_k: string) => [0.22, 1, 0.36, 1] as const,
    preferences: {
      mode: "light" as const,
      paletteId: "default",
      density: "comfortable" as const,
      fontSize: "md" as const,
      motion: "full" as const,
    },
  };
}
