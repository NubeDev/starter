import { useEffect } from 'react'
import { applyFont, applyTheme, useTheme } from '@/stores/theme-store'

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { mode, palette, font } = useTheme()

  useEffect(() => {
    applyTheme(mode, palette)
  }, [mode, palette])

  useEffect(() => {
    applyFont(font)
  }, [font])

  useEffect(() => {
    if (mode !== 'system') return
    const mql = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => applyTheme(mode, palette)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [mode, palette])

  return <>{children}</>
}
