import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import en from './en.json'
import es from './es.json'

export type Locale = 'en' | 'es'

export const LOCALES: { id: Locale; label: string }[] = [
  { id: 'en', label: 'English' },
  { id: 'es', label: 'Español' },
]

export const CATALOGS: Record<Locale, Record<string, string>> = {
  en,
  es,
}

interface LocaleState {
  locale: Locale
  setLocale: (l: Locale) => void
}

export const useLocale = create<LocaleState>()(
  persist(
    (set) => ({
      locale: 'en',
      setLocale: (locale) => set({ locale }),
    }),
    {
      name: 'test-ui-5:locale',
      storage: createJSONStorage(() => localStorage),
    },
  ),
)
