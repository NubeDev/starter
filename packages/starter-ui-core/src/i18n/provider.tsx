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
  useSyncExternalStore,
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
import {
  extensionMessagesVersion,
  getExtensionMessages,
  subscribeExtensionMessages,
} from "./extension-messages.js";
import { fetchManifest, loadCatalogCached } from "./fetcher.js";
import { I18N_FALLBACK_LANGUAGE, resolveLocale } from "./locale-fallback.js";
import { emitI18nTelemetry } from "./telemetry.js";
import type { Catalog, I18nManifest, LanguageTag } from "./types.js";

/** Hard-coded fallback language per SCOPE R5 — every starter binary
 * ships `en` and falls back to it when the requested language is
 * unknown. Re-exported from `./locale-fallback` so the resolver and
 * the provider agree on the floor. */
const FALLBACK_LANGUAGE: LanguageTag = I18N_FALLBACK_LANGUAGE;

/** Module-level "have we already fired `i18n.locale_fallback` for this
 * (requested,picked) pair this session?" guard — D-NP.6 says one
 * event per session per locale, not one per render. Lives outside
 * React state so tests can flush it via `_resetLocaleFallbackDedupeForTesting`. */
const fallbackFired = new Set<string>();

/** Test helper — wipe the per-session fallback de-dupe. */
export function _resetLocaleFallbackDedupeForTesting(): void {
  fallbackFired.clear();
}

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
  /**
   * The host's `IntlShape` from `react-intl`. Exposed here so an
   * extension reading this context through the
   * `@nube/starter-ui-core/i18n` singleton can call
   * `intl.formatMessage(...)` against the host's catalog + language
   * even when the extension bundles its own `react-intl` (in which
   * case react-intl's internal context inside the extension is empty
   * and `useIntl()` would not work). Typed as `unknown` so consumers
   * that do not pull react-intl into their type graph (the SDK) can
   * still narrow to a duck-typed call site. `null` while the first
   * catalog probe is in flight.
   */
  intl: unknown;
  /**
   * Stage-7 cross-cut. Reporter the SDK's `useHostTranslate` calls
   * when react-intl returned the key verbatim (no catalog hit). The
   * SDK does not depend on `@nube/starter-ui-core`; piping the hook
   * through the singleton handle lets the missing-key event flow
   * into the host's single `setI18nTelemetry` sink without crossing
   * the package boundary. Typed loosely so the SDK's duck-typed
   * `HostIntlContextValue` accepts the same shape.
   */
  reportMissingKey: (key: string, extensionId: string) => void;
}

/**
 * The React Context object backing `<IntlProvider>`. Exported so the
 * host's Module-Federation runtime can register it as the
 * `@nube/starter-ui-core/i18n` singleton — extensions then
 * `useContext(handle.singletons["@nube/starter-ui-core/i18n"])` against
 * the host's instance rather than bundling their own.
 */
export const IntlContext = createContext<IntlContextValue | undefined>(undefined);

export type { IntlContextValue };

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

  // Pick fingerprint for the requested language via the D-NP.6
  // left-truncating BCP-47 chain, floor `en`. The resolver also tells
  // us *how* it picked so we can emit the one `i18n.locale_fallback`
  // event per (requested,picked) per session.
  const pick = useMemo(() => {
    if (!manifest) return null;
    const r = resolveLocale(requested, manifest);
    if (!r) return null;
    if (r.fallbackUsed) {
      const dedupeKey = `${requested}${r.picked}`;
      if (!fallbackFired.has(dedupeKey)) {
        fallbackFired.add(dedupeKey);
        emitI18nTelemetry({
          kind: "i18n.locale_fallback",
          severity: "info",
          requested,
          picked: r.picked,
          chain: r.chain,
        });
      }
    }
    return { language: r.picked, fingerprint: r.fingerprint };
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

  // Extension catalogs (Stage 5 — `examples/notes/user-pref.md`). The
  // host's `extension-host.ts` lazy-fetches each enabled extension's
  // `i18n/<activeLanguage>.json` and calls `registerExtensionMessages`;
  // we subscribe via `useSyncExternalStore` so the merged bundle
  // rebuilds in the same commit as the registry mutation. The version
  // counter is monotone so React's bail-out works without a deep diff.
  const extVersion = useSyncExternalStore(
    subscribeExtensionMessages,
    extensionMessagesVersion,
    extensionMessagesVersion,
  );
  const extensionMessages = useMemo(
    () => getExtensionMessages(activeLanguage),
    // Re-read on every registry bump *and* on language switch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [activeLanguage, extVersion],
  );

  // Merge order: platform (or defaults) ← extension messages. Platform
  // keys cannot be shadowed because extension keys are namespaced by
  // extension id before they reach the registry (D-NP.3); the spread
  // order is therefore semantically irrelevant, but kept extensions-
  // last so a future debug-only override could land without
  // re-ordering.
  const messages: Record<string, string> = useMemo(
    () => ({
      ...(catalog?.messages ?? defaultMessages ?? {}),
      ...extensionMessages,
    }),
    [catalog, defaultMessages, extensionMessages],
  );

  // `intl` is built below from `(activeLanguage, messages)`; we
  // declare the context value after it so the value memo can close
  // over the `intl` instance.

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
          messages,
          onError: () => {
            /* See above — silence missing-translation warnings
             * during the prefs→catalog race. */
          },
        },
        createIntlCache(),
      ),
    [activeLanguage, messages],
  );

  const reportMissingKey = useMemo(
    () => (key: string, extensionId: string) => {
      emitI18nTelemetry({
        kind: "i18n.message_missing",
        severity: "warn",
        key,
        language: activeLanguage,
        extensionId,
      });
    },
    [activeLanguage],
  );

  const ctxValue = useMemo<IntlContextValue>(
    () => ({
      language: activeLanguage,
      manifest,
      isLoading,
      error,
      intl,
      reportMissingKey,
    }),
    [activeLanguage, manifest, isLoading, error, intl, reportMissingKey],
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
