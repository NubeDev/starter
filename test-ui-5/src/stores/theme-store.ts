import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type Mode = 'light' | 'dark' | 'system'
export type Palette = 'nube' | 'ocean' | 'sunset'
export type Font = 'geist' | 'inter' | 'manrope' | 'system'
export type Radius = 'none' | 'sm' | 'md' | 'lg'
export type Density = 'compact' | 'comfortable' | 'spacious'
export type FontSize = 'sm' | 'md' | 'lg'
export type Motion = 'full' | 'reduced'

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

export const DENSITY_SCALE: Record<Density, string> = {
  compact: '0.85',
  comfortable: '1',
  spacious: '1.15',
}

export const FONT_SIZE_PX: Record<FontSize, string> = {
  sm: '14px',
  md: '16px',
  lg: '18px',
}

export const DEFAULT_MODE: Mode = 'dark'
export const DEFAULT_PALETTE: Palette = 'nube'
export const DEFAULT_FONT: Font = 'geist'
export const DEFAULT_RADIUS: Radius = 'md'
export const DEFAULT_DENSITY: Density = 'comfortable'
export const DEFAULT_FONT_SIZE: FontSize = 'md'
export const DEFAULT_MOTION: Motion = 'full'

type State = {
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

export const useTheme = create<State>()(
  persist(
    (set) => ({
      mode: DEFAULT_MODE,
      palette: DEFAULT_PALETTE,
      font: DEFAULT_FONT,
      radius: DEFAULT_RADIUS,
      density: DEFAULT_DENSITY,
      fontSize: DEFAULT_FONT_SIZE,
      motion: DEFAULT_MOTION,
      setMode: (mode) => set({ mode }),
      setPalette: (palette) => set({ palette }),
      setFont: (font) => set({ font }),
      setRadius: (radius) => set({ radius }),
      setDensity: (density) => set({ density }),
      setFontSize: (fontSize) => set({ fontSize }),
      setMotion: (motion) => set({ motion }),
      resetTheme: () =>
        set({
          mode: DEFAULT_MODE,
          palette: DEFAULT_PALETTE,
          font: DEFAULT_FONT,
          radius: DEFAULT_RADIUS,
          density: DEFAULT_DENSITY,
          fontSize: DEFAULT_FONT_SIZE,
          motion: DEFAULT_MOTION,
        }),
    }),
    { name: 'test-ui-5-theme' },
  ),
)

export function applyFont(font: Font) {
  document.documentElement.style.setProperty('--font-sans', FONT_STACKS[font])
}

export function resolveMode(mode: Mode): 'light' | 'dark' {
  if (mode !== 'system') return mode
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export function applyTheme(mode: Mode, palette: Palette) {
  const root = document.documentElement
  root.setAttribute('data-mode', resolveMode(mode))
  root.setAttribute('data-palette', palette)
}

export function applyRadius(radius: Radius) {
  document.documentElement.style.setProperty('--radius-scale', RADIUS_SCALE[radius])
}

export function applyDensity(density: Density) {
  document.documentElement.style.setProperty('--density-scale', DENSITY_SCALE[density])
}

export function applyFontSize(size: FontSize) {
  document.documentElement.style.setProperty('--base-font-size', FONT_SIZE_PX[size])
}

export function applyMotion(motion: Motion) {
  document.documentElement.setAttribute('data-motion', motion)
}
