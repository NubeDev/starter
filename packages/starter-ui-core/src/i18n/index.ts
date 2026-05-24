// Public surface of the i18n module. Consumers import via
// `@nube/starter-ui-core/i18n` (see `package.json#exports`).

export type { Catalog, I18nManifest, LanguageTag, MessageKey } from "./types.js";

export {
  createLocaleStore,
} from "./locale-store.js";
export type {
  CreateLocaleStoreOptions,
  LocaleStoreState,
} from "./locale-store.js";

export {
  IntlContext,
  IntlProvider,
  useIntlContext,
  _resetManifestCacheForTesting,
  _resetLocaleFallbackDedupeForTesting,
} from "./provider.js";
export type { IntlContextValue, IntlProviderProps } from "./provider.js";

export {
  I18N_FALLBACK_LANGUAGE,
  resolveLocale,
} from "./locale-fallback.js";
export type { LocaleFallbackResult } from "./locale-fallback.js";

export {
  setI18nTelemetry,
  emitI18nTelemetry,
  _resetI18nTelemetryForTesting,
} from "./telemetry.js";
export type {
  I18nTelemetryEvent,
  I18nTelemetrySink,
} from "./telemetry.js";

export {
  fetchManifest,
  fetchCatalog,
  loadCatalogCached,
  _resetCatalogCacheForTesting,
} from "./fetcher.js";

export {
  registerExtensionMessages,
  unregisterExtensionMessages,
  setExtensionMessageTelemetry,
  subscribeExtensionMessages,
  extensionMessagesVersion,
  getExtensionMessages,
  _resetExtensionMessagesForTesting,
  type ExtensionMessageTelemetryEvent,
  type ExtensionMessageTelemetrySink,
  type RegisterExtensionMessagesInput,
} from "./extension-messages.js";

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
