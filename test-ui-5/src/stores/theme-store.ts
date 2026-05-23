import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type Mode = 'light' | 'dark' | 'system'
export type Palette = 'nube' | 'ocean' | 'sunset'

type State = {
  mode: Mode
  palette: Palette
  setMode: (m: Mode) => void
  setPalette: (p: Palette) => void
}

export const useTheme = create<State>()(
  persist(
    (set) => ({
      mode: 'dark',
      palette: 'nube',
      setMode: (mode) => set({ mode }),
      setPalette: (palette) => set({ palette }),
    }),
    { name: 'test-ui-5-theme' },
  ),
)

export function applyTheme(_mode: Mode, palette: Palette) {
  document.documentElement.setAttribute('data-palette', palette)
}
