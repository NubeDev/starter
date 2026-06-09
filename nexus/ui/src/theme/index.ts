// Public surface of the centralised theme module.
//
// Rebrand for a client by editing `theme.json` only — no code changes.
// Everything else (runtime, store, components) reads through here.
export {
  theme,
  palettes,
  initTheme,
  applyTheme,
  resolveMode,
  resolvePalette,
  readStoredPreference,
  readStoredPalette,
  storePreference,
  storePalette,
  THEME_STORAGE_KEY,
  PALETTE_STORAGE_KEY,
  type Theme,
  type Palette,
  type PaletteId,
  type ColorMode,
  type ThemePreference,
} from "@/theme/theme";
export { useThemeStore } from "@/theme/store";
export { ThemeProvider } from "@/theme/ThemeProvider";
export { ThemeToggle } from "@/theme/ThemeToggle";
export { PaletteSwitcher } from "@/theme/PaletteSwitcher";
