// Public theme/layout API for test-ui-5. Now sits on top of
// `@nube/starter-ui-core/theme-editor`'s `useLayoutPreferences` for
// the cross-product concerns (mode, palette, density, motion,
// fontSize) and keeps a small ui-5-only store for the two enums that
// don't generalise — `font` and `radius`.
//
// The hook shape (`useTheme`) is preserved so route components don't
// change.

import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import {
  DENSITY_SCALE as UI_DENSITY_SCALE,
  FONT_SIZE_SCALE as UI_FONT_SIZE_SCALE,
  resolveMode as resolveModeUi,
  useLayoutPreferences,
  type Density as UiDensity,
  type FontSize as UiFontSize,
  type ModePreference,
  type Motion as UiMotion,
} from '@nube/starter-ui-core/theme-editor'

// Re-export the cross-product types so the rest of the app keeps its
// existing names — and so the eventual Stage-2 cleanup is a rename,
// not a refactor.
export type Mode = ModePreference
export type Palette = 'nube' | 'ocean' | 'sunset'
export type Density = UiDensity
export type FontSize = UiFontSize
export type Motion = UiMotion

// ui-5-only enums that don't live in ui-core. Kept here.
export type Font = 'geist' | 'inter' | 'manrope' | 'system'
export type Radius = 'none' | 'sm' | 'md' | 'lg'

export const FONT_STACKS: Record<Font, string> = {
  geist: "'Geist', 'Inter', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  inter: "'Inter', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  manrope: "'Manrope', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  system: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
}

export const RADIUS_SCALE: Record<Radius, string> = {
  none: '0',
  sm: '0.5',
  md: '1',
  lg: '1.5',
}

// Local store: only the bits ui-core doesn't model.
interface LocalState {
  font: Font
  radius: Radius
  setFont: (f: Font) => void
  setRadius: (r: Radius) => void
}

export const DEFAULT_FONT: Font = 'geist'
export const DEFAULT_RADIUS: Radius = 'md'
export const DEFAULT_MODE: Mode = 'dark'
export const DEFAULT_PALETTE: Palette = 'nube'
export const DEFAULT_DENSITY: Density = 'comfortable'
export const DEFAULT_FONT_SIZE: FontSize = 'md'
export const DEFAULT_MOTION: Motion = 'full'

const useLocal = create<LocalState>()(
  persist(
    (set) => ({
      font: DEFAULT_FONT,
      radius: DEFAULT_RADIUS,
      setFont: (font) => set({ font }),
      setRadius: (radius) => set({ radius }),
    }),
    { name: 'test-ui-5-theme-local' },
  ),
)

interface UseThemeShape {
  mode: Mode
  palette: Palette
  font: Font
  radius: Radius
  density: Density
  fontSize: FontSize
  motion: Motion
  setMode: (m: Mode) => void
  setPalette: (p: Palette) => void
  setFont: (f: Font) => void
  setRadius: (r: Radius) => void
  setDensity: (d: Density) => void
  setFontSize: (s: FontSize) => void
  setMotion: (m: Motion) => void
  resetTheme: () => void
}

/** Composite hook — same shape as before, now sourced from
 * `useLayoutPreferences` (ui-core) for the shared concerns and the
 * local store for font + radius. */
export function useTheme(): UseThemeShape {
  const lp = useLayoutPreferences()
  const local = useLocal()
  return {
    mode: lp.mode,
    palette: ((lp.palette as Palette) ?? 'nube'),
    font: local.font,
    radius: local.radius,
    density: lp.density,
    fontSize: lp.fontSize,
    motion: lp.motion,
    setMode: lp.setMode,
    setPalette: (p) => lp.setPalette(p),
    setFont: local.setFont,
    setRadius: local.setRadius,
    setDensity: lp.setDensity,
    setFontSize: lp.setFontSize,
    setMotion: lp.setMotion,
    resetTheme: () => {
      lp.hydrate({
        mode: 'dark',
        density: 'comfortable',
        fontSize: 'md',
        motion: 'full',
        palette: 'nube',
      })
      local.setFont(DEFAULT_FONT)
      local.setRadius(DEFAULT_RADIUS)
    },
  }
}

// --- Apply helpers retained for compatibility ----------------------
// `ThemeProvider` now drives ui-core's `applyThemePreferences` for the
// shared concerns. These two helpers cover the ui-5-only knobs
// (font stack + radius scale) that ui-core doesn't model.

export function applyFont(font: Font) {
  document.documentElement.style.setProperty('--font-sans', FONT_STACKS[font])
}

export function applyRadius(radius: Radius) {
  document.documentElement.style.setProperty('--radius-scale', RADIUS_SCALE[radius])
}

// Re-export ui-core's resolveMode under the previous name for any
// stragglers in the codebase that still import it.
export const resolveMode = resolveModeUi

// Compatibility re-exports — components touching these names still
// build. They point at ui-core's canonical scales now.
export const DENSITY_SCALE = UI_DENSITY_SCALE
export const FONT_SIZE_SCALE = UI_FONT_SIZE_SCALE
