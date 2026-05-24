import { IntlProvider } from 'react-intl'
import type { ReactNode } from 'react'
import { CATALOGS, useLocale } from './index'

export function I18nProvider({ children }: { children: ReactNode }) {
  const locale = useLocale((s) => s.locale)
  return (
    <IntlProvider
      locale={locale}
      defaultLocale="en"
      messages={CATALOGS[locale]}
      onError={() => {
        /* suppress missing-translation warnings in dev */
      }}
    >
      {children}
    </IntlProvider>
  )
}
