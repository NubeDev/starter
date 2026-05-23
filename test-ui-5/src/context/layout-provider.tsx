import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'

export type LayoutMode = 'header' | 'sidebar'

interface LayoutContextValue {
  mode: LayoutMode
  setMode: (m: LayoutMode) => void
  toggle: () => void
}

const LayoutContext = createContext<LayoutContextValue | null>(null)

const STORAGE_KEY = 'verdant.layout'

export function LayoutProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<LayoutMode>(() => {
    if (typeof window === 'undefined') return 'header'
    const saved = window.localStorage.getItem(STORAGE_KEY)
    return saved === 'sidebar' ? 'sidebar' : 'header'
  })

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, mode)
  }, [mode])

  const setMode = (m: LayoutMode) => setModeState(m)
  const toggle = () => setModeState((m) => (m === 'header' ? 'sidebar' : 'header'))

  return (
    <LayoutContext.Provider value={{ mode, setMode, toggle }}>
      {children}
    </LayoutContext.Provider>
  )
}

export function useLayout() {
  const ctx = useContext(LayoutContext)
  if (!ctx) throw new Error('useLayout must be used inside LayoutProvider')
  return ctx
}
