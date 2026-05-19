// Stamp a token map onto a DOM element as inline custom properties.
// Used by:
//
// - The live-preview panel: scoped to a `<div data-theme-preview>` so
//   the editor can experiment without disturbing the rest of the app.
// - A consumer's runtime `applyTheme()` at startup, scoped to
//   `document.documentElement` so the saved theme takes effect.
//
// Colour values are normalised to `oklch(...)` so any author-typed
// hex / rgb / hsl input renders correctly under the
// `@nube/starter-ui-kit` stylesheet (which expects whole colour values,
// not channel triplets).

import { toOklchString } from "./color-converter.js";
import { NON_COLOR_KEYS } from "../defaults.js";
import type { ThemeStyleProps } from "../types.js";

/** Apply `props` to `element`'s inline `style.setProperty(--key, …)`.
 *
 * `mode` selects which set of properties the helper should apply (the
 * caller is responsible for swapping the two maps when the user
 * toggles light/dark). The `.dark` class is also toggled on the
 * element for components that gate on it (radix portals, sidebar). */
export function applyThemeToElement(
  element: HTMLElement,
  props: ThemeStyleProps,
  mode: "light" | "dark",
): void {
  element.classList.toggle("dark", mode === "dark");
  for (const [key, raw] of Object.entries(props)) {
    if (raw == null || raw === "") {
      element.style.removeProperty(`--${key}`);
      continue;
    }
    const value = normaliseValue(key, raw);
    element.style.setProperty(`--${key}`, value);
  }
}

/** Remove every starter-owned custom property from `element`. Used by
 * the "Reset to platform default" path so the host stylesheet's
 * defaults shine through again. */
export function clearThemeFromElement(element: HTMLElement, keys: Iterable<string>): void {
  for (const key of keys) element.style.removeProperty(`--${key}`);
}

function normaliseValue(key: string, value: string): string {
  if (NON_COLOR_KEYS.has(key)) return value;
  // Already in oklch() form — fast path, avoid the round-trip.
  if (value.startsWith("oklch(")) return value;
  return toOklchString(value) ?? value;
}
