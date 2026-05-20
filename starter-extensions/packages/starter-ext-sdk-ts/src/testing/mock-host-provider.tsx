// `MockHostProvider` — testing helper that stands up the host
// singleton handshake without booting the notes shell.
//
// Why this exists in the SDK (and not in some downstream test
// utilities package): every extension author who unit-tests their
// panel needs the same shape, and the contract is what the hooks
// (`useHostPrefs` / `useHostTranslate` / `useHostFormatters`) read
// against. Owning the helper here keeps the host's expectations and
// the test's expectations in lockstep — if the contract changes,
// the helper changes in the same PR and every consumer's test
// suite catches it.
//
// What it does, in order:
//
// 1. Creates fresh `React.Context` objects for prefs + i18n inside
//    the test (so the test never shares state with a sibling test).
// 2. Mounts those contexts' `.Provider`s with the test-supplied
//    values (resolved prefs, language, message catalogs).
// 3. Wraps everything in a `<HostBindingsProvider>` that points at
//    the freshly-created Context objects via the singleton ids the
//    SDK reads from. The hooks under test see the mock contexts
//    exactly the way they would see the host's contexts in
//    production.
//
// The helper does NOT pull `react-intl` to keep the SDK's type
// graph narrow — instead it builds a duck-typed `intl` shape with a
// flat-string formatter that mirrors the platform's "key + values"
// behaviour. Tests that need real ICU plural/select behaviour can
// pass their own `intl` override via `mockIntl`.

import * as React from "react";

import { HostBindingsProvider } from "../host-bindings.js";
import {
  SINGLETON_UI_CORE_I18N,
  SINGLETON_UI_CORE_PREFERENCES,
} from "../singleton-keys.js";
import type {
  HostIntlContextValue,
  HostIntlShape,
  HostPreferencesContextValue,
  PreferencesPatch,
  ResolvedPreferences,
} from "../prefs-types.js";

export interface MockHostProviderProps {
  /** Reverse-DNS id the SDK reports to the panel under test (used
   *  by `useHostTranslate` to auto-prefix bare keys). */
  extensionId?: string;
  /** Resolved preferences. Must include every field — the hooks
   *  throw if `preferences` is null, matching the production
   *  loading contract. */
  prefs: ResolvedPreferences;
  /** Active language tag (BCP-47). Defaults to `prefs.language`. */
  language?: string;
  /** Flat-map catalog: `{ "com.nube.hello.greeting": "Hi {name}" }`.
   *  Looked up as-is by the mock IntlShape; missing keys fall back
   *  to returning the id verbatim (matches react-intl's default). */
  catalogs?: Record<string, string>;
  /** Override the auto-built mock `IntlShape` — useful when the test
   *  needs real ICU plural/select formatting. The SDK only depends
   *  on `formatMessage`, so any object satisfying that signature
   *  works. */
  intl?: HostIntlShape;
  /** Spy/stub for `setPreferences` calls. Defaults to a no-op. */
  setPreferences?: (patch: PreferencesPatch) => Promise<void>;
  children: React.ReactNode;
}

/**
 * Mount a panel under a host-shaped tree.
 *
 * ```tsx
 * render(
 *   <MockHostProvider extensionId="com.nube.hello" prefs={AU_PREFS} catalogs={CAT}>
 *     <YourPanel />
 *   </MockHostProvider>,
 * );
 * ```
 */
export function MockHostProvider(
  props: MockHostProviderProps,
): React.ReactElement {
  const {
    extensionId = "com.test.extension",
    prefs,
    language = prefs.language,
    catalogs = {},
    intl,
    setPreferences = noopSetPreferences,
    children,
  } = props;

  // Fresh contexts per mount — guarantees test isolation. Stored in
  // refs so the Context object identity stays stable across renders
  // (re-creating it would discard React's subscription wiring).
  const contextsRef = React.useRef<MockContexts | null>(null);
  if (contextsRef.current === null) {
    contextsRef.current = {
      prefsContext: React.createContext<HostPreferencesContextValue | undefined>(undefined),
      intlContext: React.createContext<HostIntlContextValue | undefined>(undefined),
    };
  }
  const { prefsContext, intlContext } = contextsRef.current;

  const prefsValue: HostPreferencesContextValue = {
    preferences: prefs,
    isLoading: false,
    error: null,
    setPreferences,
  };

  const intlValue: HostIntlContextValue = {
    language,
    manifest: { [language]: "0000000000000000" },
    isLoading: false,
    error: null,
    intl: intl ?? buildMockIntl(catalogs),
  };

  const bindings = {
    extensionId,
    singletons: Object.freeze({
      [SINGLETON_UI_CORE_PREFERENCES]: prefsContext,
      [SINGLETON_UI_CORE_I18N]: intlContext,
    }) as Readonly<Record<string, unknown>>,
  };

  return (
    <prefsContext.Provider value={prefsValue}>
      <intlContext.Provider value={intlValue}>
        <HostBindingsProvider bindings={bindings}>{children}</HostBindingsProvider>
      </intlContext.Provider>
    </prefsContext.Provider>
  );
}

interface MockContexts {
  prefsContext: React.Context<HostPreferencesContextValue | undefined>;
  intlContext: React.Context<HostIntlContextValue | undefined>;
}

async function noopSetPreferences(): Promise<void> {
  /* test default */
}

/**
 * Build a duck-typed IntlShape that does flat-key lookup + naive
 * `{placeholder}` substitution. Sufficient for assertions that the
 * hook routed through the host's catalog; tests needing ICU plural
 * behaviour pass their own `intl` via the `intl` prop.
 */
function buildMockIntl(catalogs: Record<string, string>): HostIntlShape {
  return {
    formatMessage(descriptor, values) {
      const template = catalogs[descriptor.id];
      if (!template) return descriptor.id;
      if (!values) return template;
      return template.replace(/\{(\w+)\}/g, (_, key: string) => {
        const v = values[key];
        return v === undefined || v === null ? "" : String(v);
      });
    },
  };
}
