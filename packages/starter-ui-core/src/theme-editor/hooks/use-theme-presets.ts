// `useThemePresets` — returns the built-in preset list. Wrapped in a
// hook so a future iteration can override the gallery via context
// (e.g. consumer-supplied org-curated presets) without touching every
// call site.

import { DEFAULT_PRESETS } from "../presets.js";
import type { ThemePreset } from "../types.js";

export function useThemePresets(): readonly ThemePreset[] {
  return DEFAULT_PRESETS;
}
