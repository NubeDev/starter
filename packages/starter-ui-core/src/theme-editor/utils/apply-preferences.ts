// Stamp a `LayoutPreferences` snapshot onto a DOM element via data-*
// attributes + CSS variables.
//
// This is intentionally separate from `applyThemeToElement`:
//   - `applyThemeToElement` writes inline custom properties for the 38
//     theme tokens — it owns the colour/typography/shape model.
//   - `applyThemePreferences` writes runtime presentation knobs that
//     the consumer's CSS (Tailwind v4 `@theme` blocks, classic CSS
//     custom properties, whatever) hooks onto.
//
// Selectors a consumer typically wires up:
//   - `[data-mode="dark"]`  — toggled by mode resolution
//   - `[data-palette="<id>"]`
//   - `[data-motion="reduced"]`
//   - `--density-scale` (number)
//   - `--font-size-scale` (number)

import {
  DENSITY_SCALE,
  FONT_SIZE_SCALE,
  resolveMode,
  type LayoutPreferences,
  type ResolvedMode,
} from "../layout-preferences.js";

export interface ApplyPreferencesResult {
  /** The mode that was actually rendered (after `system` resolution). */
  resolvedMode: ResolvedMode;
}

/** Apply a `LayoutPreferences` snapshot to `element`. The element is
 * usually `document.documentElement`.
 *
 * Returns the resolved mode so callers that also need to drive
 * `applyThemeToElement` (which takes `"light" | "dark"`) can chain
 * without re-resolving. */
export function applyThemePreferences(
  element: HTMLElement,
  prefs: LayoutPreferences,
): ApplyPreferencesResult {
  const resolvedMode = resolveMode(prefs.mode);

  element.setAttribute("data-mode", resolvedMode);
  element.classList.toggle("dark", resolvedMode === "dark");

  if (prefs.palette) {
    element.setAttribute("data-palette", prefs.palette);
  } else {
    element.removeAttribute("data-palette");
  }

  element.setAttribute("data-motion", prefs.motion);
  element.setAttribute("data-density", prefs.density);
  element.setAttribute("data-font-size", prefs.fontSize);

  element.style.setProperty("--density-scale", String(DENSITY_SCALE[prefs.density]));
  element.style.setProperty("--font-size-scale", String(FONT_SIZE_SCALE[prefs.fontSize]));

  return { resolvedMode };
}

/** Remove every preference attribute / variable this helper writes.
 * Used by the "reset to platform default" path. */
export function clearThemePreferences(element: HTMLElement): void {
  element.removeAttribute("data-mode");
  element.classList.remove("dark");
  element.removeAttribute("data-palette");
  element.removeAttribute("data-motion");
  element.removeAttribute("data-density");
  element.removeAttribute("data-font-size");
  element.style.removeProperty("--density-scale");
  element.style.removeProperty("--font-size-scale");
}
