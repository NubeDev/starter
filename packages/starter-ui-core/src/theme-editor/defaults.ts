// Built-in defaults. These mirror the `:root` and `.dark` blocks in
// `@nube/starter-ui-kit/src/styles/globals.css` exactly — when the
// editor "Reset to platform default" button is pressed, we restore
// these values verbatim so the rendered result matches a fresh install
// with no overrides.
//
// If `globals.css` ever changes, update these constants in lockstep
// (they are duplicated by design — the kit owns the CSS; this package
// owns the data shape).

import type { ThemeStyleProps, ThemeStyles } from "./types.js";

export const defaultLightThemeStyles: ThemeStyleProps = {
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

export const defaultDarkThemeStyles: ThemeStyleProps = {
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

export const defaultThemeStyles: ThemeStyles = {
  light: defaultLightThemeStyles,
  dark: defaultDarkThemeStyles,
};

/** Tokens that are not colours and must pass through the apply path
 * verbatim (no OKLCH conversion). Exposed for `apply-theme.ts`. */
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
