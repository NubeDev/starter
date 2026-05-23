import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type Mode = 'light' | 'dark' | 'system'
export type Palette = 'nube' | 'ocean' | 'sunset'
export type Font = 'geist' | 'inter' | 'manrope' | 'system'

export const FONT_STACKS: Record<Font, string> = {
  geist: "'Geist', 'Inter', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  inter: "'Inter', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  manrope: "'Manrope', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  system: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
}

export const DEFAULT_MODE: Mode = 'dark'
export const DEFAULT_PALETTE: Palette = 'nube'
export const DEFAULT_FONT: Font = 'geist'

type State = {
  mode: Mode
  palette: Palette
  font: Font
  setMode: (m: Mode) => void
  setPalette: (p: Palette) => void
  setFont: (f: Font) => void
  resetTheme: () => void
}

export const useTheme = create<State>()(
  persist(
    (set) => ({
      mode: DEFAULT_MODE,
      palette: DEFAULT_PALETTE,
      font: DEFAULT_FONT,
      setMode: (mode) => set({ mode }),
      setPalette: (palette) => set({ palette }),
      setFont: (font) => set({ font }),
      resetTheme: () =>
        set({ mode: DEFAULT_MODE, palette: DEFAULT_PALETTE, font: DEFAULT_FONT }),
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
