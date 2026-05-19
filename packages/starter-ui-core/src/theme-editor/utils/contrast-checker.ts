// WCAG 2.x contrast-ratio helper. Used by `ColorTokenEditor` to badge
// each foreground/background pair with AAA / AA / Fail.
//
// Adapted from tweakcn (https://github.com/jnsahaj/tweakcn).
// Original work Copyright (c) 2024 Sahaj Jain. Apache License 2.0.
// Modifications Copyright (c) starter contributors.

import { parse, toRgb } from "./color-converter.js";

/** WCAG contrast tier returned by [`getContrastTier`]. */
export type ContrastTier = "AAA" | "AA" | "fail";

/** Compute the WCAG 2.x contrast ratio between two colours. Returns
 * `null` if either input fails to parse — callers should display "—"
 * rather than a misleading number. */
export function getContrastRatio(a: string, b: string): number | null {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  if (la == null || lb == null) return null;
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

/** Map a numeric ratio to a WCAG tier. The thresholds are the standard
 * "normal text" cutoffs (AA ≥ 4.5, AAA ≥ 7). */
export function getContrastTier(ratio: number | null): ContrastTier {
  if (ratio == null) return "fail";
  if (ratio >= 7) return "AAA";
  if (ratio >= 4.5) return "AA";
  return "fail";
}

/** Internal: WCAG relative luminance for a CSS colour string. */
function relativeLuminance(input: string): number | null {
  const parsed = parse(input);
  if (!parsed) return null;
  const rgb = toRgb(parsed);
  if (!rgb) return null;
  const r = channel(rgb.r ?? 0);
  const g = channel(rgb.g ?? 0);
  const b = channel(rgb.b ?? 0);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function channel(v: number): number {
  return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}
