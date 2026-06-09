// Centralised theme runtime.
//
// The look is driven by two JSON files, both client-editable with no
// TypeScript changes:
//
//   • `theme.json`     — brand meta: name, the first-visit mode, and the
//                        default palette id.
//   • `palettes.json`  — the user-switchable colour systems. Each palette
//                        is a full light + dark token map keyed by the
//                        platform's `ThemeStyleKey` set.
//
// Two independent axes the user controls at runtime:
//   • mode    — light | dark | system   (ThemeToggle)
//   • palette — emerald | blue | violet (PaletteSwitcher)
//
// The kit maps every `--color-* → var(--*)`, so writing the bare `--*`
// custom properties onto `<html>` re-skins every shadcn primitive,
// ECharts series, the sidebar, the aurora backdrop, and react-grid-layout
// chrome at once — no component edits, no rebuild.

import type { ThemeStyleKey, ThemeTokenMap } from "@nube/starter-theme-tokens";

import themeJson from "./theme.json";
import palettesJson from "./palettes.json";

/** What the user picks for the dark/light axis. */
export type ColorMode = "light" | "dark";
export type ThemePreference = ColorMode | "system";

/** A single switchable colour system: a full light + dark token map. */
export interface Palette {
  id: string;
  name: string;
  description?: string;
  light: ThemeTokenMap;
  dark: ThemeTokenMap;
}

/** Brand meta from `theme.json`. */
export interface Theme {
  name: string;
  description?: string;
  defaultMode: ThemePreference;
  defaultPalette: string;
  tokens: {
    light: ThemeTokenMap;
    dark: ThemeTokenMap;
  };
}

/** The active brand meta, loaded from JSON. */
export const theme = themeJson as Theme;

/** All user-selectable palettes, in menu order. */
export const palettes = (palettesJson as { palettes: Palette[] }).palettes;

/** Narrow string of valid palette ids (stable keys, persisted). */
export type PaletteId = string;

/** localStorage keys for the user's persisted choices. */
export const THEME_STORAGE_KEY = "nexus.theme.mode";
export const PALETTE_STORAGE_KEY = "nexus.theme.palette";

/** The palette to use when none is stored / a stored id is unknown. */
function fallbackPalette(): Palette {
  return (
    palettes.find((p) => p.id === theme.defaultPalette) ?? palettes[0]
  );
}

/** Resolve an id to a concrete palette, falling back to the default. */
export function resolvePalette(id: PaletteId): Palette {
  return palettes.find((p) => p.id === id) ?? fallbackPalette();
}

/** Resolve a preference to a concrete mode, consulting the OS for
 * "system". Safe to call before React mounts. */
export function resolveMode(pref: ThemePreference): ColorMode {
  if (pref === "system") {
    return typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return pref;
}

/** Read the persisted mode preference, else the theme's default. */
export function readStoredPreference(): ThemePreference {
  if (typeof window === "undefined") return theme.defaultMode;
  const raw = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (raw === "light" || raw === "dark" || raw === "system") return raw;
  return theme.defaultMode;
}

/** Read the persisted palette id, else the theme's default. Always a
 * valid, known id. */
export function readStoredPalette(): PaletteId {
  if (typeof window === "undefined") return fallbackPalette().id;
  const raw = window.localStorage.getItem(PALETTE_STORAGE_KEY);
  if (raw && palettes.some((p) => p.id === raw)) return raw;
  return fallbackPalette().id;
}

/** Persist the user's mode preference. */
export function storePreference(pref: ThemePreference): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(THEME_STORAGE_KEY, pref);
}

/** Persist the user's palette choice. */
export function storePalette(id: PaletteId): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(PALETTE_STORAGE_KEY, id);
}

/**
 * Write a palette's token map for `mode` onto `<html>` as `--<key>`
 * custom properties and flip the `.dark` class + `color-scheme` so the
 * kit's dark variant and native form controls follow. This is the only
 * place the DOM is touched for theming.
 */
export function applyTheme(mode: ColorMode, paletteId?: PaletteId): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const palette = resolvePalette(paletteId ?? readStoredPalette());
  const tokens = palette[mode];

  for (const [key, value] of Object.entries(tokens) as [
    ThemeStyleKey,
    string,
  ][]) {
    root.style.setProperty(`--${key}`, value);
  }

  root.classList.toggle("dark", mode === "dark");
  root.style.colorScheme = mode;
  root.dataset.theme = theme.name.toLowerCase();
  root.dataset.palette = palette.id;
}

/**
 * One-shot bootstrap: resolve the stored mode + palette and paint them
 * before React renders, so there's no flash of the wrong look. Returns
 * the resolved values so callers can seed their store. Call once from the
 * app entry, before `createRoot`.
 */
export function initTheme(): {
  preference: ThemePreference;
  mode: ColorMode;
  palette: PaletteId;
} {
  const preference = readStoredPreference();
  const mode = resolveMode(preference);
  const palette = readStoredPalette();
  applyTheme(mode, palette);
  return { preference, mode, palette };
}
