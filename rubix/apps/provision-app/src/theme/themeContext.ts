import { createContext, useContext } from 'react'
import type { AppTheme, ThemeKey } from './themes'
import type { StatusKey } from './statuses'

// Theme context + hook live here (not in the provider component file) so the
// provider module only exports a component — keeps Fast Refresh happy and obeys
// the one-concept-per-file rule.
export interface ThemeState {
  themeKey: ThemeKey
  theme: AppTheme
  setThemeKey: (k: ThemeKey) => void
  // live connection/device status — tints the accent (transient, not persisted)
  status: StatusKey | null
  setStatus: (s: StatusKey | null) => void
}

export const ThemeContext = createContext<ThemeState | null>(null)

export function useAppTheme() {
  const ctx = useContext(ThemeContext)
  if (!ctx) throw new Error('useAppTheme must be used within ThemeProvider')
  return ctx
}
