// i18n/provider.tsx — react-intl wrapper.
//
// APP-SHELL.md §Provider stack item 7: same `en.json` / `es.json` shape
// the web app uses, react-intl is the v1 engine. Locale resolution
// reads the OS preference; persisting an operator override lives in
// AsyncStorage and lands in a follow-up (it's a 6th provider, not
// blocking the slice).

import { type ReactNode, useMemo } from 'react';
import { IntlProvider } from 'react-intl';
import { NativeModules, Platform } from 'react-native';

import en from './en.json';
import es from './es.json';

type Messages = Record<string, string>;

const CATALOGUES: Record<string, Messages> = {
  en,
  es,
};

function deviceLocale(): string {
  // Expo SDK 52+ ships full Intl; the OS locale is the obvious default.
  // We hand-roll the lookup so we don't pull `expo-localization` for one
  // string (every kB matters on a cold start).
  if (Platform.OS === 'ios') {
    const settings =
      NativeModules.SettingsManager?.settings?.AppleLocale ??
      NativeModules.SettingsManager?.settings?.AppleLanguages?.[0];
    if (typeof settings === 'string') return settings;
  }
  if (Platform.OS === 'android') {
    const tag = NativeModules.I18nManager?.localeIdentifier;
    if (typeof tag === 'string') return tag;
  }
  return 'en';
}

function resolveCatalogue(tag: string): { locale: string; messages: Messages } {
  const lang = tag.split(/[_-]/)[0]?.toLowerCase() ?? 'en';
  if (lang in CATALOGUES) {
    return { locale: lang, messages: CATALOGUES[lang]! };
  }
  return { locale: 'en', messages: CATALOGUES.en! };
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const { locale, messages } = useMemo(() => resolveCatalogue(deviceLocale()), []);
  return (
    <IntlProvider locale={locale} defaultLocale="en" messages={messages}>
      {children}
    </IntlProvider>
  );
}
