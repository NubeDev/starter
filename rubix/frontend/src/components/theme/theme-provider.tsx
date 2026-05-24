import { useEffect } from 'react'
import {
  applyThemePreferences,
  subscribePrefersDark,
  useLayoutPreferences,
} from '@nube/starter-ui-core/theme-editor'
import {
  applyFont,
  applyRadius,
  useTheme,
} from '@/stores/theme-store'

/** Single React effect driver. ui-core's `applyThemePreferences`
 * handles mode + palette + density + motion + font-size in one
 * write; we still call the ui-5-only helpers for `font` and
 * `radius`. */
export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const prefs = useLayoutPreferences()
  const { font, radius, mode } = useTheme()

  useEffect(() => {
    applyThemePreferences(document.documentElement, {
      mode: prefs.mode,
      density: prefs.density,
      fontSize: prefs.fontSize,
      motion: prefs.motion,
      palette: prefs.palette ?? 'nube',
    })
  }, [prefs.mode, prefs.density, prefs.fontSize, prefs.motion, prefs.palette])

  useEffect(() => {
    applyFont(font)
  }, [font])

  useEffect(() => {
    applyRadius(radius)
  }, [radius])

  // Re-apply when the OS-level prefers-color-scheme flips, but only
  // when the user picked "system".
  useEffect(() => {
    if (mode !== 'system') return
    return subscribePrefersDark(() => {
      applyThemePreferences(document.documentElement, {
        mode: prefs.mode,
        density: prefs.density,
        fontSize: prefs.fontSize,
        motion: prefs.motion,
        palette: prefs.palette ?? 'nube',
      })
    })
  }, [mode, prefs.mode, prefs.density, prefs.fontSize, prefs.motion, prefs.palette])

  return <>{children}</>
}
