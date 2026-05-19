// Colour-space conversion helpers. Thin wrapper over `culori` so the
// rest of the editor never imports `culori` directly — if we ever swap
// the colour library, only this file changes.
//
// Adapted from tweakcn (https://github.com/jnsahaj/tweakcn).
// Original work Copyright (c) 2024 Sahaj Jain. Apache License 2.0.
// Modifications Copyright (c) starter contributors.

import {
  converter,
  formatHex,
  formatRgb,
  parse,
  type Color,
  type Rgb,
} from "culori";

const toOklch = converter("oklch");
// Explicit return-shape annotation keeps the inferred type
// portable (TS2742 fires without it because `Converter<...>` references
// types under `culori/src/converter` that aren't part of the public
// type surface).
const toRgb: (color: string | Color | undefined) => Rgb | undefined = converter("rgb");

/** Output formats the editor understands. */
export type ColorFormat = "hex" | "rgb" | "oklch";

/** Parse any CSS colour string (hex, rgb(), hsl(), oklch(), named) and
 * return its OKLCH form as `oklch(L C H)`. Returns `null` if the input
 * does not parse — callers display the raw string in the input field
 * so the user can see what's wrong without losing their edit. */
export function toOklchString(input: string): string | null {
  const parsed = parse(input);
  if (!parsed) return null;
  const o = toOklch(parsed);
  if (!o) return null;
  const l = round(o.l ?? 0, 4);
  const c = round(o.c ?? 0, 4);
  const h = round(o.h ?? 0, 2);
  const alpha = o.alpha != null && o.alpha < 1 ? ` / ${round(o.alpha, 3)}` : "";
  return `oklch(${l} ${c} ${h}${alpha})`;
}

/** Render the colour in the requested format, or `null` if the input
 * doesn't parse. Used by the colour-picker swatch to give the user
 * their pick of formats when copying. */
export function colorFormatter(input: string, format: ColorFormat): string | null {
  const parsed = parse(input);
  if (!parsed) return null;
  switch (format) {
    case "hex":
      return formatHex(parsed) ?? null;
    case "rgb":
      return formatRgb(parsed) ?? null;
    case "oklch":
      return toOklchString(input);
  }
}

/** Convert any CSS colour string to a `#rrggbb` hex value the native
 * `<input type="color">` will accept. Returns `#000000` on parse failure
 * so the picker still renders (the editor still warns the user via the
 * text field). */
export function toHexForPicker(input: string): string {
  const parsed = parse(input);
  if (!parsed) return "#000000";
  return formatHex(parsed) ?? "#000000";
}

/** Internal: rounded numeric format helper. */
function round(value: number, digits: number): number {
  const m = 10 ** digits;
  return Math.round(value * m) / m;
}

/** Re-export the raw `Color` type for callers that want it without
 * pulling `culori` into their import surface. */
export type { Color };
/** Re-export `parse` and the rgb converter for internal use by the
 * contrast-checker. Not part of the public surface. */
export { parse, toRgb };
