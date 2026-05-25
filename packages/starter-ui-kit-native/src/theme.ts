// `useTheme()` — the single token-access hook every primitive uses.
//
// Resolves the active mode (light/dark/system → resolved) + palette
// from the layout-preferences store published by
// `@nube/starter-ui-core/theme-editor`, looks up the matching palette
// in `@nube/starter-theme-tokens` (`NAMED_PALETTES`), and exposes a
// flat token bag the primitives consume.
//
// MUST NOT touch DOM. MUST NOT import `starter-ui-kit`. MUST be
// usable from React Native (no `window`-only code on the hot path —
// the store already guards `matchMedia`).

import {
  DURATION_MS,
  EASING,
  FONT_SIZE_REM,
  FONT_WEIGHT,
  NAMED_PALETTES,
  RADIUS_BASE_REM,
  RADIUS_MULTIPLIERS,
  ROLE_TO_TOKENS,
  SPACING_BASE_REM,
  SPACING_SCALE,
  platformDarkPalette,
  platformLightPalette,
  type RadiusSize,
  type Role,
  type ThemeStyleKey,
  type ThemeTokenMap,
} from "@nube/starter-theme-tokens";
import {
  resolveMode,
  useLayoutPreferences,
  type ResolvedMode,
} from "@nube/starter-ui-core/theme-editor";

/** The subset of the layout-prefs state the kit reads on every render. */
export interface ThemePreferencesSnapshot {
  mode: ResolvedMode;
  paletteId: string;
  density: "compact" | "comfortable" | "spacious";
  fontSize: "sm" | "md" | "lg";
  motion: "full" | "reduced";
}

/** Flat token bag returned by `useTheme()`. The shape is intentionally
 * boring — primitives index by string, the type guarantees keys exist. */
export interface Theme {
  /** Resolved colour mode (`"light" | "dark"`). */
  mode: ResolvedMode;
  /** Named palette id resolved against `NAMED_PALETTES`. */
  paletteId: string;
  /** Raw token map for the active mode. Keys are `ThemeStyleKey`. */
  colors: Readonly<ThemeTokenMap>;
  /** Look up a colour by token key, falling back to platform default. */
  color: (key: ThemeStyleKey) => string;
  /** Look up a (background, foreground, border) triple for a Role. */
  role: (role: Role) => {
    background: string;
    foreground: string;
    border: string | undefined;
  };
  /** Spacing helper — `space(2)` = 0.5rem in px (assume 16px root). */
  space: (step: keyof typeof SPACING_SCALE | number) => number;
  /** Radius helper — `radius("md")` returns px. */
  radius: (size: RadiusSize | "base") => number;
  /** Type scale helper — `fontSize("base")` returns px. */
  fontSize: (size: keyof typeof FONT_SIZE_REM) => number;
  /** Font weight helper — `fontWeight("medium")` returns a numeric weight. */
  fontWeight: (weight: keyof typeof FONT_WEIGHT) => number;
  /** Motion helpers (returns 0 ms when user requested reduced motion). */
  duration: (key: keyof typeof DURATION_MS) => number;
  /** Easing tuple suitable for `Easing.bezier(...)`. */
  easing: (key: keyof typeof EASING) => readonly [number, number, number, number];
  /** Raw preferences snapshot. */
  preferences: ThemePreferencesSnapshot;
}

const PX_PER_REM = 16;

/** rem → px. Pulled out so a future high-DPI scaling factor lands in
 * one place. */
function rem(value: number): number {
  return Math.round(value * PX_PER_REM);
}

function lookupPalette(paletteId: string | null): {
  light: ThemeTokenMap;
  dark: ThemeTokenMap;
} {
  if (paletteId) {
    const hit = NAMED_PALETTES.find((p) => p.id === paletteId);
    if (hit) return hit.styles;
  }
  return { light: platformLightPalette, dark: platformDarkPalette };
}

/** Read the active token bag. Re-renders when the layout-preferences
 * store updates — same instance the web theme-editor writes to. */
export function useTheme(): Theme {
  const prefs = useLayoutPreferences();
  const mode = resolveMode(prefs.mode);
  const paletteId = prefs.palette ?? "platform-default";
  const palette = lookupPalette(prefs.palette);
  const colors = mode === "dark" ? palette.dark : palette.light;
  const fallback =
    mode === "dark" ? platformDarkPalette : platformLightPalette;
  const densityScale =
    prefs.density === "compact" ? 0.85 : prefs.density === "spacious" ? 1.15 : 1;
  const fontScale =
    prefs.fontSize === "sm" ? 0.875 : prefs.fontSize === "lg" ? 1.125 : 1;
  const motionMul = prefs.motion === "reduced" ? 0 : 1;

  const color = (key: ThemeStyleKey): string =>
    (colors[key] ?? fallback[key] ?? "transparent") as string;

  return {
    mode,
    paletteId,
    colors,
    color,
    role: (r) => {
      const tokens = ROLE_TO_TOKENS[r];
      return {
        background: color(tokens.background),
        foreground: color(tokens.foreground),
        border: tokens.border ? color(tokens.border) : undefined,
      };
    },
    space: (step) => {
      const n =
        typeof step === "number" ? step : (SPACING_SCALE[step] ?? Number(step));
      return Math.round(SPACING_BASE_REM * PX_PER_REM * n * densityScale);
    },
    radius: (size) => {
      if (size === "base") return rem(RADIUS_BASE_REM);
      const mul = RADIUS_MULTIPLIERS[size] ?? 1;
      return rem(RADIUS_BASE_REM * mul);
    },
    fontSize: (size) => Math.round(rem(FONT_SIZE_REM[size]) * fontScale),
    fontWeight: (weight) => FONT_WEIGHT[weight],
    duration: (key) => DURATION_MS[key] * motionMul,
    easing: (key) => EASING[key],
    preferences: {
      mode,
      paletteId,
      density: prefs.density,
      fontSize: prefs.fontSize,
      motion: prefs.motion,
    },
  };
}
