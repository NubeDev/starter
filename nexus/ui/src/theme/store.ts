import { create } from "zustand";

import {
  applyTheme,
  readStoredPalette,
  readStoredPreference,
  resolveMode,
  storePalette,
  storePreference,
  type ColorMode,
  type PaletteId,
  type ThemePreference,
} from "@/theme/theme";
import { invalidateThemeCache } from "@/features/widgets/palette";

// Re-apply tokens to <html>, drop ECharts' cached (canvas) colours so
// charts re-tint, and return the mode. Every mutator funnels through
// here so the DOM, the chart cache, and React state never drift.
function paint(mode: ColorMode, palette: PaletteId): ColorMode {
  applyTheme(mode, palette);
  invalidateThemeCache();
  return mode;
}

// Reactive theme state. Two independent axes:
//   • `preference`/`mode` — the dark/light choice (mode is what's painted)
//   • `palette`           — the active colour system id
// Every mutator re-applies the resolved tokens to `<html>` and persists
// the choice, so the DOM, localStorage, and React stay in lock-step.
//
// `create` is imported from the workspace's single `zustand` — the same
// federation singleton the rest of the app uses (see `store/ui.ts`) — so
// extensions reading the theme share one store runtime.
interface ThemeState {
  preference: ThemePreference;
  mode: ColorMode;
  palette: PaletteId;
  /** Set an explicit preference ("light" | "dark" | "system"). */
  setPreference: (pref: ThemePreference) => void;
  /** Flip light↔dark, pinning the result as an explicit preference. */
  toggle: () => void;
  /** Switch the active colour system, keeping the current mode. */
  setPalette: (id: PaletteId) => void;
  /** Re-resolve from the OS — used when "system" and the OS flips. */
  syncSystem: () => void;
}

const initialPreference = readStoredPreference();
const initialPalette = readStoredPalette();

export const useThemeStore = create<ThemeState>((set, get) => ({
  preference: initialPreference,
  mode: resolveMode(initialPreference),
  palette: initialPalette,

  setPreference: (preference) => {
    const mode = paint(resolveMode(preference), get().palette);
    storePreference(preference);
    set({ preference, mode });
  },

  toggle: () => {
    const next = paint(
      get().mode === "dark" ? "light" : "dark",
      get().palette,
    );
    storePreference(next);
    set({ preference: next, mode: next });
  },

  setPalette: (palette) => {
    paint(get().mode, palette);
    storePalette(palette);
    set({ palette });
  },

  syncSystem: () => {
    if (get().preference !== "system") return;
    set({ mode: paint(resolveMode("system"), get().palette) });
  },
}));
