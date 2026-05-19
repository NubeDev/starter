// Curated preset gallery. Ten themes adapted from tweakcn (Apache-2.0,
// https://github.com/jnsahaj/tweakcn) — see the attribution block on
// each value tuple. Only the public colour data is reused; no source
// code from tweakcn is in this file.
//
// To add a preset: append a `ThemePreset` to `DEFAULT_PRESETS`. The
// editor picks it up automatically.

import { defaultDarkThemeStyles, defaultLightThemeStyles } from "./defaults.js";
import type { ThemePreset, ThemeStyleProps } from "./types.js";

// Adapted from tweakcn (https://github.com/jnsahaj/tweakcn).
// Original work Copyright (c) 2024 Sahaj Jain. Apache License 2.0.
// Modifications Copyright (c) starter contributors.

/** Merge a partial token override on top of the platform defaults so
 * every preset still produces a complete, renderable theme even if its
 * author only specified the half-dozen tokens that differ. */
function withLight(overrides: ThemeStyleProps): ThemeStyleProps {
  return { ...defaultLightThemeStyles, ...overrides };
}
function withDark(overrides: ThemeStyleProps): ThemeStyleProps {
  return { ...defaultDarkThemeStyles, ...overrides };
}

export const DEFAULT_PRESETS: readonly ThemePreset[] = [
  {
    id: "platform-default",
    label: "Platform Default",
    description: "The out-of-the-box starter theme.",
    styles: {
      light: defaultLightThemeStyles,
      dark: defaultDarkThemeStyles,
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
