// Public surface of the i18n module. Consumers import via
// `@nube/starter-ui-core/i18n` (see `package.json#exports`).

export type { Catalog, I18nManifest, LanguageTag, MessageKey } from "./types.js";

export {
  IntlProvider,
  useIntlContext,
  _resetManifestCacheForTesting,
} from "./provider.js";
export type { IntlProviderProps } from "./provider.js";

export {
  fetchManifest,
  fetchCatalog,
  loadCatalogCached,
  _resetCatalogCacheForTesting,
} from "./fetcher.js";

export { useTranslate } from "./use-translate.js";
export type {
  AppMessageKeys,
  MessageValues,
  TranslateFn,
} from "./use-translate.js";

// Re-export the Settings page from here too, so the most common
// consumer wiring (`import { SettingsPage, IntlProvider, useTranslate }
// from "@nube/starter-ui-core/i18n"`) works in one import.
export { SettingsPage } from "../preferences/SettingsPage.js";
export type { SettingsPageProps, ToastFn } from "../preferences/SettingsPage.js";
