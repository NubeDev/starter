import { createLocaleStore } from '@nube/starter-ui-core/i18n'
import en from './en.json'
import es from './es.json'

const LOCALE_IDS = ['en', 'es'] as const
export type Locale = (typeof LOCALE_IDS)[number]

export const LOCALES: { id: Locale; label: string }[] = [
  { id: 'en', label: 'English' },
  { id: 'es', label: 'Español' },
]

export const CATALOGS: Record<Locale, Record<string, string>> = {
  en,
  es,
}

export const useLocale = createLocaleStore({
  persistKey: 'test-ui-5:locale',
  locales: LOCALE_IDS,
  defaultLocale: 'en',
})
