import { createContext, useContext, useState, type ReactNode } from 'react'
import { getCookie, setCookie } from '@/lib/cookies'

export type LayoutMode = 'header' | 'sidebar'
export type Collapsible = 'offcanvas' | 'icon' | 'none'
export type Variant = 'inset' | 'sidebar' | 'floating'

interface LayoutContextValue {
  defaultMode: LayoutMode
  mode: LayoutMode
  setMode: (m: LayoutMode) => void
  toggle: () => void

  defaultCollapsible: Collapsible
  collapsible: Collapsible
  setCollapsible: (c: Collapsible) => void

  defaultVariant: Variant
  variant: Variant
  setVariant: (v: Variant) => void

  resetLayout: () => void
}

const LayoutContext = createContext<LayoutContextValue | null>(null)

const COOKIE_MODE = 'layout_mode'
const COOKIE_COLLAPSIBLE = 'layout_collapsible'
const COOKIE_VARIANT = 'layout_variant'
const MAX_AGE = 60 * 60 * 24 * 365

const DEFAULT_MODE: LayoutMode = 'header'
const DEFAULT_COLLAPSIBLE: Collapsible = 'icon'
const DEFAULT_VARIANT: Variant = 'floating'

function readCookie<T extends string>(name: string, fallback: T, allowed: readonly T[]): T {
  const v = getCookie(name)
  return v && (allowed as readonly string[]).includes(v) ? (v as T) : fallback
}

const MODES: readonly LayoutMode[] = ['header', 'sidebar']
const COLLAPSIBLES: readonly Collapsible[] = ['offcanvas', 'icon', 'none']
const VARIANTS: readonly Variant[] = ['inset', 'sidebar', 'floating']

export function LayoutProvider({ children }: { children: ReactNode }) {
  const [mode, _setMode] = useState<LayoutMode>(() => readCookie(COOKIE_MODE, DEFAULT_MODE, MODES))
  const [collapsible, _setCollapsible] = useState<Collapsible>(() =>
    readCookie(COOKIE_COLLAPSIBLE, DEFAULT_COLLAPSIBLE, COLLAPSIBLES),
  )
  const [variant, _setVariant] = useState<Variant>(() =>
    readCookie(COOKIE_VARIANT, DEFAULT_VARIANT, VARIANTS),
  )

  const setMode = (m: LayoutMode) => {
    setCookie(COOKIE_MODE, m, MAX_AGE)
    _setMode(m)
  }
  const setCollapsible = (c: Collapsible) => {
    setCookie(COOKIE_COLLAPSIBLE, c, MAX_AGE)
    _setCollapsible(c)
  }
  const setVariant = (v: Variant) => {
    setCookie(COOKIE_VARIANT, v, MAX_AGE)
    _setVariant(v)
  }

  const toggle = () => setMode(mode === 'header' ? 'sidebar' : 'header')

  const resetLayout = () => {
    setMode(DEFAULT_MODE)
    setCollapsible(DEFAULT_COLLAPSIBLE)
    setVariant(DEFAULT_VARIANT)
  }

  return (
    <LayoutContext.Provider
      value={{
        defaultMode: DEFAULT_MODE,
        mode,
        setMode,
        toggle,
        defaultCollapsible: DEFAULT_COLLAPSIBLE,
        collapsible,
        setCollapsible,
        defaultVariant: DEFAULT_VARIANT,
        variant,
        setVariant,
        resetLayout,
      }}
    >
      {children}
    </LayoutContext.Provider>
  )
}

// eslint-disable-next-line react-refresh/only-export-components
export function useLayout() {
  const ctx = useContext(LayoutContext)
  if (!ctx) throw new Error('useLayout must be used inside LayoutProvider')
  return ctx
}
