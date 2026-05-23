import { createContext, useContext, useEffect, useState } from 'react'
import { DirectionProvider as RdxDirProvider } from '@radix-ui/react-direction'
import { getCookie, removeCookie, setCookie } from '@/lib/cookies'

export type Direction = 'ltr' | 'rtl'

const DEFAULT_DIRECTION: Direction = 'ltr'
const COOKIE = 'dir'
const MAX_AGE = 60 * 60 * 24 * 365

type DirectionContextType = {
  defaultDir: Direction
  dir: Direction
  setDir: (dir: Direction) => void
  resetDir: () => void
}

const DirectionContext = createContext<DirectionContextType | null>(null)

export function DirectionProvider({ children }: { children: React.ReactNode }) {
  const [dir, _setDir] = useState<Direction>(
    () => (getCookie(COOKIE) as Direction) || DEFAULT_DIRECTION,
  )

  useEffect(() => {
    document.documentElement.setAttribute('dir', dir)
  }, [dir])

  const setDir = (d: Direction) => {
    _setDir(d)
    setCookie(COOKIE, d, MAX_AGE)
  }

  const resetDir = () => {
    _setDir(DEFAULT_DIRECTION)
    removeCookie(COOKIE)
  }

  return (
    <DirectionContext.Provider value={{ defaultDir: DEFAULT_DIRECTION, dir, setDir, resetDir }}>
      <RdxDirProvider dir={dir}>{children}</RdxDirProvider>
    </DirectionContext.Provider>
  )
}

// eslint-disable-next-line react-refresh/only-export-components
export function useDirection() {
  const ctx = useContext(DirectionContext)
  if (!ctx) throw new Error('useDirection must be used within a DirectionProvider')
  return ctx
}
