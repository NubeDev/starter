import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'

export type HeroVariant = 'glass' | 'shader'

interface HeroVariantContextValue {
  variant: HeroVariant
  setVariant: (v: HeroVariant) => void
  toggle: () => void
}

const HeroVariantContext = createContext<HeroVariantContextValue | null>(null)
const STORAGE_KEY = 'verdant.hero'

export function HeroVariantProvider({ children }: { children: ReactNode }) {
  const [variant, setVariantState] = useState<HeroVariant>(() => {
    if (typeof window === 'undefined') return 'shader'
    const saved = window.localStorage.getItem(STORAGE_KEY)
    return saved === 'glass' ? 'glass' : 'shader'
  })

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, variant)
  }, [variant])

  return (
    <HeroVariantContext.Provider
      value={{
        variant,
        setVariant: setVariantState,
        toggle: () => setVariantState((v) => (v === 'glass' ? 'shader' : 'glass')),
      }}
    >
      {children}
    </HeroVariantContext.Provider>
  )
}

export function useHeroVariant() {
  const ctx = useContext(HeroVariantContext)
  if (!ctx) throw new Error('useHeroVariant must be used inside HeroVariantProvider')
  return ctx
}
