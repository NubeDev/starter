import { useEffect } from 'react'
import { applyTheme, useTheme } from '@/stores/theme-store'

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { mode, palette } = useTheme()
  useEffect(() => { applyTheme(mode, palette) }, [mode, palette])
  return <>{children}</>
}
