import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'

export type LayoutMode = 'header' | 'sidebar'
export type Collapsible = 'offcanvas' | 'icon' | 'none'
export type Variant = 'inset' | 'sidebar' | 'floating'

interface LayoutContextValue {
  mode: LayoutMode
  setMode: (m: LayoutMode) => void
  toggle: () => void

  collapsible: Collapsible
  setCollapsible: (c: Collapsible) => void

  variant: Variant
  setVariant: (v: Variant) => void
}

const LayoutContext = createContext<LayoutContextValue | null>(null)

const STORAGE_KEY = 'verdant.layout'
const DEFAULT_COLLAPSIBLE: Collapsible = 'icon'
const DEFAULT_VARIANT: Variant = 'floating'

export function LayoutProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<LayoutMode>(() => {
    if (typeof window === 'undefined') return 'header'
    const saved = window.localStorage.getItem(STORAGE_KEY)
    return saved === 'sidebar' ? 'sidebar' : 'header'
  })

  const [collapsible, setCollapsible] = useState<Collapsible>(DEFAULT_COLLAPSIBLE)
  const [variant, setVariant] = useState<Variant>(DEFAULT_VARIANT)

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, mode)
  }, [mode])

  const setMode = (m: LayoutMode) => setModeState(m)
  const toggle = () => setModeState((m) => (m === 'header' ? 'sidebar' : 'header'))

  return (
    <LayoutContext.Provider
      value={{ mode, setMode, toggle, collapsible, setCollapsible, variant, setVariant }}
    >
      {children}
    </LayoutContext.Provider>
  )
}

export function useLayout() {
  const ctx = useContext(LayoutContext)
  if (!ctx) throw new Error('useLayout must be used inside LayoutProvider')
  return ctx
}
