// `<IntlProvider>` — wraps `react-intl`'s IntlProvider and feeds it
// the catalog for the active language. The active language is read
// from `<PreferencesProvider>` (`prefs.language`); when prefs aren't
// available yet (initial mount, error) we fall back to `en`.
//
// Lifecycle:
// 1. On mount, GET `/v1/i18n/manifest` (cached for the session via
//    `loadManifestOnce`).
// 2. Pick the fingerprint for `prefs.language` from the manifest;
//    fall back to `en` if the requested language is not advertised.
// 3. Fetch the fingerprinted catalog through the immutable cache
//    (`loadCatalogCached`).
// 4. Render `react-intl`'s `IntlProvider` with `{ locale, messages }`.
//    A language switch triggers a remount of the provider (keyed on
//    `locale`) so react-intl rebuilds its internal caches.

import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { RawIntlProvider as RawIntlProviderUntyped, createIntl, createIntlCache } from "react-intl";
import type { ComponentType, ProviderProps } from "react";
import type { IntlShape } from "react-intl";

// react-intl 7 pulls in @types/react 19 transitively; under
// @types/react 18 the `ReactNode` shapes don't line up and TS rejects
// `<RawIntlProvider>` as a JSX element. Re-narrow to the React-18
// `ComponentType` we actually consume. Pure typing — runtime is the
// same react-intl export.
const RawIntlProvider = RawIntlProviderUntyped as unknown as ComponentType<ProviderProps<IntlShape>>;
import type { StarterClient } from "@nube/starter-client-ts";

import { usePreferences } from "../preferences/provider.js";
import { fetchManifest, loadCatalogCached } from "./fetcher.js";
import type { Catalog, I18nManifest, LanguageTag } from "./types.js";

/** Hard-coded fallback language per SCOPE R5 — every starter binary
 * ships `en` and falls back to it when the requested language is
 * unknown. */
const FALLBACK_LANGUAGE: LanguageTag = "en";

/** Module-level memoised manifest fetch. The manifest is small and
 * the fingerprints inside are content-addressed, so caching for the
 * session is safe. */
let manifestPromise: Promise<I18nManifest> | null = null;
function loadManifestOnce(client: StarterClient): Promise<I18nManifest> {
  if (!manifestPromise) {
    manifestPromise = fetchManifest(client).catch((err) => {
      manifestPromise = null;
      throw err;
    });
  }
  return manifestPromise;
}

/** Test helper — wipe the module-level manifest promise. */
export function _resetManifestCacheForTesting(): void {
  manifestPromise = null;
}

interface CatalogState {
  language: LanguageTag;
  messages: Catalog;
}

interface IntlContextValue {
  /** Currently-active language (the one passed to react-intl). */
  language: LanguageTag;
  /** Manifest, once loaded. `null` while in flight. */
  manifest: I18nManifest | null;
  /** True while the initial catalog probe is in flight. */
  isLoading: boolean;
  /** Last fetch error, if any. */
  error: unknown;
}

const IntlContext = createContext<IntlContextValue | undefined>(undefined);

export interface IntlProviderProps {
  /** Shared `StarterClient`. */
  client: StarterClient;
  /** Override the active language. When omitted, the language is
   * read from `<PreferencesProvider>` (`prefs.language`). Useful for
   * Storybook and for apps that mount Intl above Preferences. */
  language?: LanguageTag;
  /** Default messages to seed react-intl with before the network
   * catalog arrives. Avoids a flash of message-keys at startup. */
  defaultMessages?: Catalog;
  children: ReactNode;
}

/** Wire the i18n catalog into a React subtree. */
export function IntlProvider({
  client,
  language: languageOverride,
  defaultMessages,
  children,
}: IntlProviderProps) {
  // Pref-driven language; gracefully degrade when no PreferencesProvider
  // is mounted so callers can use IntlProvider standalone.
  const prefsLanguage = useOptionalPrefsLanguage();
  const requested = languageOverride ?? prefsLanguage ?? FALLBACK_LANGUAGE;

  const [manifest, setManifest] = useState<I18nManifest | null>(null);
  const [catalog, setCatalog] = useState<CatalogState | null>(
    defaultMessages ? { language: FALLBACK_LANGUAGE, messages: defaultMessages } : null,
  );
  const [error, setError] = useState<unknown>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Load the manifest once per session.
  useEffect(() => {
    let cancelled = false;
    loadManifestOnce(client)
      .then((m) => {
        if (!cancelled) setManifest(m);
      })
      .catch((err) => {
        if (!cancelled) setError(err);
      });
    return () => {
      cancelled = true;
    };
  }, [client]);

  // Pick fingerprint for the requested language; fall back to en.
  const pick = useMemo(() => {
    if (!manifest) return null;
    if (manifest[requested]) {
      return { language: requested, fingerprint: manifest[requested]! };
    }
    if (manifest[FALLBACK_LANGUAGE]) {
      return {
        language: FALLBACK_LANGUAGE,
        fingerprint: manifest[FALLBACK_LANGUAGE]!,
      };
    }
    return null;
  }, [manifest, requested]);

  // Load the catalog for the picked (language, fingerprint).
  useEffect(() => {
    if (!pick) return;
    let cancelled = false;
    setIsLoading(true);
    loadCatalogCached(client, pick.language, pick.fingerprint)
      .then((messages) => {
        if (!cancelled) {
          setCatalog({ language: pick.language, messages });
          setIsLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err);
          setIsLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [client, pick]);

  const activeLanguage = catalog?.language ?? FALLBACK_LANGUAGE;
  const messages = catalog?.messages ?? defaultMessages ?? {};

  const ctxValue = useMemo<IntlContextValue>(
    () => ({
      language: activeLanguage,
      manifest,
      isLoading,
      error,
    }),
    [activeLanguage, manifest, isLoading, error],
  );

  // Build an `IntlShape` directly so we sidestep react-intl's class-
  // component `<IntlProvider>` (whose typings don't line up with
  // @types/react 18). RawIntlProvider just plumbs the shape through
  // context — equivalent behaviour for our purposes. We rebuild on
  // every (locale, messages) change; the cache key + the
  // `key={activeLanguage}` on RawIntlProvider together force the
  // remount-on-language-switch behaviour the SCOPE calls for.
  const intl = useMemo(
    () =>
      createIntl(
        {
          locale: activeLanguage,
          defaultLocale: FALLBACK_LANGUAGE,
          messages: messages as Record<string, string>,
          onError: () => {
            /* See above — silence missing-translation warnings
             * during the prefs→catalog race. */
          },
        },
        createIntlCache(),
      ),
    [activeLanguage, messages],
  );

  return (
    <IntlContext.Provider value={ctxValue}>
      <RawIntlProvider key={activeLanguage} value={intl}>
        {children}
      </RawIntlProvider>
    </IntlContext.Provider>
  );
}

/** Read the IntlProvider context (loading flags, manifest). */
export function useIntlContext(): IntlContextValue {
  const ctx = useContext(IntlContext);
  if (!ctx) throw new Error("useIntlContext must be called inside <IntlProvider>");
  return ctx;
}

/** Read `prefs.language` if a `<PreferencesProvider>` is mounted;
 * return `undefined` otherwise. We swallow the missing-provider
 * error so IntlProvider can be used standalone (Storybook, tests). */
function useOptionalPrefsLanguage(): LanguageTag | undefined {
  // The hook is always called (rules-of-hooks compliant); the throw
  // path is only taken when no PreferencesProvider is mounted.
  try {
    return usePreferences().preferences?.language;
  } catch {
    return undefined;
  }
}
