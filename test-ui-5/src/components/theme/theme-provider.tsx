import { useEffect } from 'react'
import {
  applyDensity,
  applyFont,
  applyFontSize,
  applyMotion,
  applyRadius,
  applyTheme,
  useTheme,
} from '@/stores/theme-store'

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { mode, palette, font, radius, density, fontSize, motion } = useTheme()

  useEffect(() => {
    applyTheme(mode, palette)
  }, [mode, palette])

  useEffect(() => {
    applyFont(font)
  }, [font])

  useEffect(() => {
    applyRadius(radius)
  }, [radius])

  useEffect(() => {
    applyDensity(density)
  }, [density])

  useEffect(() => {
    applyFontSize(fontSize)
  }, [fontSize])

  useEffect(() => {
    applyMotion(motion)
  }, [motion])

  useEffect(() => {
    if (mode !== 'system') return
    const mql = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => applyTheme(mode, palette)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [mode, palette])

  return <>{children}</>
}
