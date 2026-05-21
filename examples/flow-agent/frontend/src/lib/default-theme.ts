// Flow-agent's custom default theme.
//
// We don't ship a CSS override file; instead we stamp these token maps
// onto `document.documentElement` at boot via the theme-editor's
// `applyThemeToElement`. That keeps the kit stylesheet authoritative
// while letting this app present its own out-of-the-box brand.
//
// Inspired by macOS Tahoe / iOS 2026: cool near-white surfaces, a
// confident system-blue primary, slightly larger radii, softer borders.

import {
  defaultDarkThemeStyles,
  defaultLightThemeStyles,
  type ThemeStyleProps,
} from "@nube/starter-ui-core/theme-editor"

export const flowAgentLightTheme: ThemeStyleProps = {
  ...defaultLightThemeStyles,
  background: "oklch(0.985 0.004 250)",
  foreground: "oklch(0.18 0.015 255)",
  card: "oklch(1 0 0)",
  "card-foreground": "oklch(0.18 0.015 255)",
  popover: "oklch(1 0 0)",
  "popover-foreground": "oklch(0.18 0.015 255)",
  primary: "oklch(0.6 0.2 255)",
  "primary-foreground": "oklch(0.99 0 0)",
  secondary: "oklch(0.96 0.01 255)",
  "secondary-foreground": "oklch(0.25 0.02 255)",
  muted: "oklch(0.965 0.006 250)",
  "muted-foreground": "oklch(0.5 0.02 250)",
  accent: "oklch(0.94 0.03 255)",
  "accent-foreground": "oklch(0.35 0.15 255)",
  border: "oklch(0.92 0.006 250)",
  input: "oklch(0.92 0.006 250)",
  ring: "oklch(0.6 0.2 255)",
  sidebar: "oklch(0.978 0.005 250)",
  "sidebar-foreground": "oklch(0.18 0.015 255)",
  "sidebar-primary": "oklch(0.6 0.2 255)",
  "sidebar-primary-foreground": "oklch(0.99 0 0)",
  "sidebar-accent": "oklch(0.94 0.03 255)",
  "sidebar-accent-foreground": "oklch(0.35 0.15 255)",
  "sidebar-border": "oklch(0.92 0.006 250)",
  "sidebar-ring": "oklch(0.6 0.2 255)",
  radius: "0.75rem",
  "font-sans":
    '-apple-system, BlinkMacSystemFont, "SF Pro Text", "Inter", ui-sans-serif, system-ui, sans-serif',
}

export const flowAgentDarkTheme: ThemeStyleProps = {
  ...defaultDarkThemeStyles,
  background: "oklch(0.16 0.012 250)",
  foreground: "oklch(0.97 0.003 250)",
  card: "oklch(0.21 0.014 250)",
  "card-foreground": "oklch(0.97 0.003 250)",
  popover: "oklch(0.21 0.014 250)",
  "popover-foreground": "oklch(0.97 0.003 250)",
  primary: "oklch(0.7 0.18 255)",
  "primary-foreground": "oklch(0.15 0.02 255)",
  secondary: "oklch(0.28 0.018 250)",
  "secondary-foreground": "oklch(0.97 0.003 250)",
  muted: "oklch(0.26 0.014 250)",
  "muted-foreground": "oklch(0.72 0.014 250)",
  accent: "oklch(0.32 0.07 255)",
  "accent-foreground": "oklch(0.95 0.04 255)",
  border: "oklch(1 0 0 / 8%)",
  input: "oklch(1 0 0 / 12%)",
  ring: "oklch(0.7 0.18 255)",
  sidebar: "oklch(0.19 0.014 250)",
  "sidebar-foreground": "oklch(0.97 0.003 250)",
  "sidebar-primary": "oklch(0.7 0.18 255)",
  "sidebar-primary-foreground": "oklch(0.15 0.02 255)",
  "sidebar-accent": "oklch(0.32 0.07 255)",
  "sidebar-accent-foreground": "oklch(0.95 0.04 255)",
  "sidebar-border": "oklch(1 0 0 / 8%)",
  "sidebar-ring": "oklch(0.7 0.18 255)",
  radius: "0.75rem",
  "font-sans":
    '-apple-system, BlinkMacSystemFont, "SF Pro Text", "Inter", ui-sans-serif, system-ui, sans-serif',
}
