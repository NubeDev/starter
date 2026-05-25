// Platform palette — source of truth.
//
// Two consumers read this file:
//
//   1. `packages/starter-ui-kit/scripts/generate-css.ts` emits the
//      `:root` and `.dark` blocks of `globals.css` from
//      `platformLightPalette` / `platformDarkPalette` in the exact
//      order listed in `CSS_EMISSION_ORDER` (subset of the full
//      token map — non-colour fields like `font-sans` are owned by
//      `type.ts` and not emitted into `globals.css` as CSS vars).
//
//   2. `packages/starter-ui-core/src/theme-editor/defaults.ts` and
//      `presets.ts` re-export from here so the theme editor's
//      "Reset to platform default" produces values that match a
//      fresh kit install bit-for-bit.
//
// No React, no DOM, no styling runtime — just data.

/** Every editable token key recognised by the theme editor. */
export type ThemeStyleKey =
  // Colour — paired light/dark, but the key itself is mode-agnostic.
  | "background"
  | "foreground"
  | "card"
  | "card-foreground"
  | "popover"
  | "popover-foreground"
  | "primary"
  | "primary-foreground"
  | "secondary"
  | "secondary-foreground"
  | "muted"
  | "muted-foreground"
  | "accent"
  | "accent-foreground"
  | "destructive"
  | "destructive-foreground"
  | "border"
  | "input"
  | "ring"
  | "chart-1"
  | "chart-2"
  | "chart-3"
  | "chart-4"
  | "chart-5"
  | "sidebar"
  | "sidebar-foreground"
  | "sidebar-primary"
  | "sidebar-primary-foreground"
  | "sidebar-accent"
  | "sidebar-accent-foreground"
  | "sidebar-border"
  | "sidebar-ring"
  // Shape.
  | "radius"
  // Typography.
  | "font-sans"
  | "font-serif"
  | "font-mono"
  | "letter-spacing"
  // Shadow.
  | "shadow-color"
  | "shadow-opacity"
  | "shadow-blur"
  | "shadow-spread"
  | "shadow-offset-x"
  | "shadow-offset-y";

/** A complete (or partial) token map for one mode. */
export type ThemeTokenMap = Partial<Record<ThemeStyleKey, string>>;

/** Light + dark variant pair. */
export interface ThemePalette {
  light: ThemeTokenMap;
  dark: ThemeTokenMap;
}

/** Named preset palette (for the theme editor gallery). */
export interface NamedPalette {
  id: string;
  label: string;
  description: string;
  styles: ThemePalette;
}

/* -------------------------------------------------------------------------
 * Platform defaults (mirror `:root` + `.dark` in
 * `packages/starter-ui-kit/src/styles/globals.css`).
 * ----------------------------------------------------------------------- */

export const platformLightPalette: ThemeTokenMap = {
  background: "oklch(1 0 0)",
  foreground: "oklch(0.148 0.004 228.8)",
  card: "oklch(1 0 0)",
  "card-foreground": "oklch(0.148 0.004 228.8)",
  popover: "oklch(1 0 0)",
  "popover-foreground": "oklch(0.148 0.004 228.8)",
  primary: "oklch(0.218 0.008 223.9)",
  "primary-foreground": "oklch(0.987 0.002 197.1)",
  secondary: "oklch(0.963 0.002 197.1)",
  "secondary-foreground": "oklch(0.218 0.008 223.9)",
  muted: "oklch(0.963 0.002 197.1)",
  "muted-foreground": "oklch(0.56 0.021 213.5)",
  accent: "oklch(0.963 0.002 197.1)",
  "accent-foreground": "oklch(0.218 0.008 223.9)",
  destructive: "oklch(0.577 0.245 27.325)",
  "destructive-foreground": "oklch(0.987 0.002 197.1)",
  border: "oklch(0.925 0.005 214.3)",
  input: "oklch(0.925 0.005 214.3)",
  ring: "oklch(0.723 0.014 214.4)",
  "chart-1": "oklch(0.872 0.007 219.6)",
  "chart-2": "oklch(0.56 0.021 213.5)",
  "chart-3": "oklch(0.45 0.017 213.2)",
  "chart-4": "oklch(0.378 0.015 216)",
  "chart-5": "oklch(0.275 0.011 216.9)",
  radius: "0.625rem",
  sidebar: "oklch(0.987 0.002 197.1)",
  "sidebar-foreground": "oklch(0.148 0.004 228.8)",
  "sidebar-primary": "oklch(0.218 0.008 223.9)",
  "sidebar-primary-foreground": "oklch(0.987 0.002 197.1)",
  "sidebar-accent": "oklch(0.963 0.002 197.1)",
  "sidebar-accent-foreground": "oklch(0.218 0.008 223.9)",
  "sidebar-border": "oklch(0.925 0.005 214.3)",
  "sidebar-ring": "oklch(0.723 0.014 214.4)",
  "font-sans": "ui-sans-serif, system-ui, sans-serif",
  "font-serif": "ui-serif, Georgia, serif",
  "font-mono": "ui-monospace, SFMono-Regular, Menlo, monospace",
  "letter-spacing": "0em",
  "shadow-color": "oklch(0 0 0)",
  "shadow-opacity": "0.1",
  "shadow-blur": "10px",
  "shadow-spread": "0px",
  "shadow-offset-x": "0px",
  "shadow-offset-y": "4px",
};

export const platformDarkPalette: ThemeTokenMap = {
  background: "oklch(0.148 0.004 228.8)",
  foreground: "oklch(0.987 0.002 197.1)",
  card: "oklch(0.218 0.008 223.9)",
  "card-foreground": "oklch(0.987 0.002 197.1)",
  popover: "oklch(0.218 0.008 223.9)",
  "popover-foreground": "oklch(0.987 0.002 197.1)",
  primary: "oklch(0.925 0.005 214.3)",
  "primary-foreground": "oklch(0.218 0.008 223.9)",
  secondary: "oklch(0.275 0.011 216.9)",
  "secondary-foreground": "oklch(0.987 0.002 197.1)",
  muted: "oklch(0.275 0.011 216.9)",
  "muted-foreground": "oklch(0.723 0.014 214.4)",
  accent: "oklch(0.275 0.011 216.9)",
  "accent-foreground": "oklch(0.987 0.002 197.1)",
  destructive: "oklch(0.704 0.191 22.216)",
  "destructive-foreground": "oklch(0.987 0.002 197.1)",
  border: "oklch(1 0 0 / 10%)",
  input: "oklch(1 0 0 / 15%)",
  ring: "oklch(0.56 0.021 213.5)",
  "chart-1": "oklch(0.872 0.007 219.6)",
  "chart-2": "oklch(0.56 0.021 213.5)",
  "chart-3": "oklch(0.45 0.017 213.2)",
  "chart-4": "oklch(0.378 0.015 216)",
  "chart-5": "oklch(0.275 0.011 216.9)",
  radius: "0.625rem",
  sidebar: "oklch(0.218 0.008 223.9)",
  "sidebar-foreground": "oklch(0.987 0.002 197.1)",
  "sidebar-primary": "oklch(0.488 0.243 264.376)",
  "sidebar-primary-foreground": "oklch(0.987 0.002 197.1)",
  "sidebar-accent": "oklch(0.275 0.011 216.9)",
  "sidebar-accent-foreground": "oklch(0.987 0.002 197.1)",
  "sidebar-border": "oklch(1 0 0 / 10%)",
  "sidebar-ring": "oklch(0.56 0.021 213.5)",
  "font-sans": "ui-sans-serif, system-ui, sans-serif",
  "font-serif": "ui-serif, Georgia, serif",
  "font-mono": "ui-monospace, SFMono-Regular, Menlo, monospace",
  "letter-spacing": "0em",
  "shadow-color": "oklch(0 0 0)",
  "shadow-opacity": "0.3",
  "shadow-blur": "10px",
  "shadow-spread": "0px",
  "shadow-offset-x": "0px",
  "shadow-offset-y": "4px",
};

export const platformPalette: ThemePalette = {
  light: platformLightPalette,
  dark: platformDarkPalette,
};

/** Tokens that are not colours and must pass through the theme-editor
 * apply path verbatim (no OKLCH conversion). */
export const NON_COLOR_KEYS: ReadonlySet<string> = new Set([
  "radius",
  "font-sans",
  "font-serif",
  "font-mono",
  "letter-spacing",
  "shadow-opacity",
  "shadow-blur",
  "shadow-spread",
  "shadow-offset-x",
  "shadow-offset-y",
]);

/* -------------------------------------------------------------------------
 * CSS emission order — used by the kit's generate-css script. Subset
 * of ThemeStyleKey: only the keys that today appear as `--var: …;`
 * lines in `globals.css`. Non-colour tokens that the kit doesn't
 * surface as CSS vars (font-sans, shadow-*, …) are deliberately
 * absent here; they remain in the theme-editor data model.
 * ----------------------------------------------------------------------- */

export const CSS_EMISSION_ORDER_LIGHT: readonly ThemeStyleKey[] = [
  "background",
  "foreground",
  "card",
  "card-foreground",
  "popover",
  "popover-foreground",
  "primary",
  "primary-foreground",
  "secondary",
  "secondary-foreground",
  "muted",
  "muted-foreground",
  "accent",
  "accent-foreground",
  "destructive",
  "border",
  "input",
  "ring",
  "chart-1",
  "chart-2",
  "chart-3",
  "chart-4",
  "chart-5",
  "radius",
  "sidebar",
  "sidebar-foreground",
  "sidebar-primary",
  "sidebar-primary-foreground",
  "sidebar-accent",
  "sidebar-accent-foreground",
  "sidebar-border",
  "sidebar-ring",
];

/** Same as light, minus `radius` (dark inherits the light radius). */
export const CSS_EMISSION_ORDER_DARK: readonly ThemeStyleKey[] =
  CSS_EMISSION_ORDER_LIGHT.filter((k) => k !== "radius");

/* -------------------------------------------------------------------------
 * Named preset palettes — gallery entries for the theme editor.
 *
 * Adapted from tweakcn (https://github.com/jnsahaj/tweakcn).
 * Original work Copyright (c) 2024 Sahaj Jain. Apache License 2.0.
 * Modifications Copyright (c) starter contributors.
 *
 * Each preset's light/dark map is a *partial* override applied on top
 * of `platformLightPalette` / `platformDarkPalette`. Authors only list
 * the half-dozen tokens that differ; consumers must merge.
 * ----------------------------------------------------------------------- */

function withLight(overrides: ThemeTokenMap): ThemeTokenMap {
  return { ...platformLightPalette, ...overrides };
}
function withDark(overrides: ThemeTokenMap): ThemeTokenMap {
  return { ...platformDarkPalette, ...overrides };
}

export const NAMED_PALETTES: readonly NamedPalette[] = [
  {
    id: "platform-default",
    label: "Platform Default",
    description: "The out-of-the-box starter theme.",
    styles: {
      light: platformLightPalette,
      dark: platformDarkPalette,
    },
  },
  {
    id: "modern-minimal",
    label: "Modern Minimal",
    description: "Neutral surfaces with a confident blue accent.",
    styles: {
      light: withLight({
        primary: "oklch(0.58 0.22 257)",
        "primary-foreground": "oklch(0.99 0 0)",
        ring: "oklch(0.58 0.22 257)",
        radius: "0.5rem",
      }),
      dark: withDark({
        primary: "oklch(0.7 0.2 257)",
        "primary-foreground": "oklch(0.15 0.02 257)",
        ring: "oklch(0.7 0.2 257)",
        radius: "0.5rem",
      }),
    },
  },
  {
    id: "violet-bloom",
    label: "Violet Bloom",
    description: "Soft violet primary on warm neutrals.",
    styles: {
      light: withLight({
        primary: "oklch(0.55 0.22 295)",
        "primary-foreground": "oklch(0.99 0 0)",
        accent: "oklch(0.94 0.04 295)",
        "accent-foreground": "oklch(0.35 0.18 295)",
        ring: "oklch(0.55 0.22 295)",
        radius: "0.875rem",
      }),
      dark: withDark({
        primary: "oklch(0.72 0.18 295)",
        "primary-foreground": "oklch(0.15 0.02 295)",
        accent: "oklch(0.3 0.08 295)",
        "accent-foreground": "oklch(0.95 0.04 295)",
        ring: "oklch(0.72 0.18 295)",
        radius: "0.875rem",
      }),
    },
  },
  {
    id: "kodama-grove",
    label: "Kodama Grove",
    description: "Mossy green with a parchment background.",
    styles: {
      light: withLight({
        background: "oklch(0.96 0.02 90)",
        foreground: "oklch(0.25 0.03 145)",
        primary: "oklch(0.45 0.13 145)",
        "primary-foreground": "oklch(0.98 0.02 90)",
        accent: "oklch(0.9 0.06 90)",
        "accent-foreground": "oklch(0.3 0.1 145)",
        ring: "oklch(0.45 0.13 145)",
        radius: "0.5rem",
      }),
      dark: withDark({
        background: "oklch(0.2 0.02 145)",
        primary: "oklch(0.7 0.15 145)",
        "primary-foreground": "oklch(0.15 0.02 145)",
        ring: "oklch(0.7 0.15 145)",
        radius: "0.5rem",
      }),
    },
  },
  {
    id: "neo-brutalism",
    label: "Neo Brutalism",
    description: "Hard edges, high contrast, no apology.",
    styles: {
      light: withLight({
        background: "oklch(1 0 0)",
        foreground: "oklch(0 0 0)",
        primary: "oklch(0.7 0.25 30)",
        "primary-foreground": "oklch(0 0 0)",
        border: "oklch(0 0 0)",
        ring: "oklch(0 0 0)",
        radius: "0rem",
      }),
      dark: withDark({
        background: "oklch(0.1 0 0)",
        foreground: "oklch(1 0 0)",
        primary: "oklch(0.75 0.25 30)",
        "primary-foreground": "oklch(0 0 0)",
        border: "oklch(1 0 0)",
        ring: "oklch(1 0 0)",
        radius: "0rem",
      }),
    },
  },
  {
    id: "cosmic-night",
    label: "Cosmic Night",
    description: "Deep indigo surfaces, default to dark mode.",
    styles: {
      light: withLight({
        primary: "oklch(0.45 0.2 270)",
        "primary-foreground": "oklch(0.99 0 0)",
        ring: "oklch(0.45 0.2 270)",
      }),
      dark: withDark({
        background: "oklch(0.16 0.03 270)",
        card: "oklch(0.2 0.04 270)",
        primary: "oklch(0.72 0.18 270)",
        "primary-foreground": "oklch(0.15 0.02 270)",
        accent: "oklch(0.3 0.08 270)",
        "accent-foreground": "oklch(0.95 0.04 270)",
        ring: "oklch(0.72 0.18 270)",
      }),
    },
  },
  {
    id: "elegant-luxury",
    label: "Elegant Luxury",
    description: "Champagne gold on charcoal.",
    styles: {
      light: withLight({
        background: "oklch(0.98 0.01 85)",
        primary: "oklch(0.55 0.13 75)",
        "primary-foreground": "oklch(0.99 0.01 85)",
        accent: "oklch(0.92 0.05 75)",
        "accent-foreground": "oklch(0.35 0.13 75)",
        ring: "oklch(0.55 0.13 75)",
        radius: "0.25rem",
      }),
      dark: withDark({
        background: "oklch(0.12 0.01 75)",
        primary: "oklch(0.78 0.13 75)",
        "primary-foreground": "oklch(0.12 0.01 75)",
        accent: "oklch(0.25 0.05 75)",
        "accent-foreground": "oklch(0.9 0.06 75)",
        ring: "oklch(0.78 0.13 75)",
        radius: "0.25rem",
      }),
    },
  },
  {
    id: "amber-minimal",
    label: "Amber Minimal",
    description: "Warm amber accent on clean neutrals.",
    styles: {
      light: withLight({
        primary: "oklch(0.72 0.18 65)",
        "primary-foreground": "oklch(0.2 0.02 65)",
        accent: "oklch(0.95 0.05 65)",
        "accent-foreground": "oklch(0.4 0.13 65)",
        ring: "oklch(0.72 0.18 65)",
        radius: "0.75rem",
      }),
      dark: withDark({
        primary: "oklch(0.78 0.18 65)",
        "primary-foreground": "oklch(0.18 0.02 65)",
        accent: "oklch(0.3 0.07 65)",
        "accent-foreground": "oklch(0.95 0.05 65)",
        ring: "oklch(0.78 0.18 65)",
        radius: "0.75rem",
      }),
    },
  },
  {
    id: "ocean-breeze",
    label: "Ocean Breeze",
    description: "Teal primary with airy blue surfaces.",
    styles: {
      light: withLight({
        background: "oklch(0.98 0.01 210)",
        primary: "oklch(0.55 0.13 195)",
        "primary-foreground": "oklch(0.99 0 0)",
        accent: "oklch(0.92 0.05 195)",
        "accent-foreground": "oklch(0.35 0.13 195)",
        ring: "oklch(0.55 0.13 195)",
        radius: "1rem",
      }),
      dark: withDark({
        background: "oklch(0.16 0.03 210)",
        primary: "oklch(0.72 0.13 195)",
        "primary-foreground": "oklch(0.15 0.02 195)",
        accent: "oklch(0.28 0.07 195)",
        "accent-foreground": "oklch(0.95 0.04 195)",
        ring: "oklch(0.72 0.13 195)",
        radius: "1rem",
      }),
    },
  },
  {
    id: "soft-pop",
    label: "Soft Pop",
    description: "Pastel pink with a generous corner radius.",
    styles: {
      light: withLight({
        primary: "oklch(0.7 0.18 0)",
        "primary-foreground": "oklch(0.99 0 0)",
        accent: "oklch(0.95 0.05 340)",
        "accent-foreground": "oklch(0.4 0.15 0)",
        ring: "oklch(0.7 0.18 0)",
        radius: "1.25rem",
      }),
      dark: withDark({
        primary: "oklch(0.78 0.18 0)",
        "primary-foreground": "oklch(0.18 0.02 0)",
        accent: "oklch(0.3 0.07 340)",
        "accent-foreground": "oklch(0.95 0.05 340)",
        ring: "oklch(0.78 0.18 0)",
        radius: "1.25rem",
      }),
    },
  },
];
