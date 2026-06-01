import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { THEMES, DEFAULT_THEME, type ThemeKey } from './themes'
import type { StatusKey } from './statuses'
import { ThemeContext, type ThemeState } from './themeContext'

const STORAGE_KEY = 'rbx.provision.theme'

function readStored(): ThemeKey {
  try {
    const v = localStorage.getItem(STORAGE_KEY)
    if (v && v in THEMES) return v as ThemeKey
  } catch {
    /* ignore — private mode */
  }
  return DEFAULT_THEME
}

// Holds the long-term theme (persisted) + the transient device status (not
// persisted, since it reflects the current session's connection state).
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [themeKey, setThemeKey] = useState<ThemeKey>(readStored)
  const [status, setStatus] = useState<StatusKey | null>(null)

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, themeKey)
    } catch {
      /* ignore */
    }
  }, [themeKey])

  const value = useMemo<ThemeState>(
    () => ({ themeKey, theme: THEMES[themeKey], setThemeKey, status, setStatus }),
    [themeKey, status],
  )

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}
