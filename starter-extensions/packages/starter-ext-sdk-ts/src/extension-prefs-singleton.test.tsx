// Stage-3 smoke test: the SDK's prefs + i18n hook surface routes
// through the singleton handshake correctly.
//
// What this proves:
//
// 1. `useHostPrefs()` returns the resolved preferences from the
//    `@nube/starter-ui-core/preferences` singleton — not a duplicate
//    fetch.
// 2. `useHostFormatters()` produces the same en-AU + BBQ-°F output
//    the host chrome would produce (Stage 1 smoke), proving the
//    extension shares one source of truth with the host.
// 3. `useHostTranslate()` calls the host's IntlShape and auto-
//    prefixes bare keys with the extension id (D-NP.3).
// 4. The hooks throw with the documented messages when their
//    wiring contract is violated (no provider, no singleton).
// 5. `registerExtensionContributions` wraps registered components in
//    a `<HostBindingsProvider>` seeded from the handle, so a panel
//    rendered through the registration path resolves the hooks
//    without the extension author wiring anything by hand.
// 6. `MockHostProvider` (subpath: `@nube/starter-ext-sdk-ts/testing`)
//    stands the same shape up in unit tests without booting the
//    notes host.

import { afterEach, describe, expect, it, vi } from "vitest";
import * as React from "react";
import { cleanup, render, screen } from "@testing-library/react";

import {
  registerExtensionContributions,
  useHostFormatters,
  useHostPrefs,
  useHostTranslate,
  type ExtensionRemoteHandle,
  type HostIntlContextValue,
  type HostPreferencesContextValue,
  type ResolvedPreferences,
  SINGLETON_UI_CORE_I18N,
  SINGLETON_UI_CORE_PREFERENCES,
} from "./index.js";
import { HostBindingsProvider } from "./host-bindings.js";
import { MockHostProvider } from "./testing/index.js";

afterEach(() => {
  cleanup();
});

/** en-AU operator with a BBQ override on the temperature unit. The
 *  same fixture the Stage 1 host smoke uses, so the test pins
 *  "host chrome and extension agree" down to the byte. */
const AU_PREFS: ResolvedPreferences = {
  timezone: "Australia/Brisbane",
  locale: "en-AU",
  language: "en-AU",
  unit_system: "metric",
  temperature_unit: "fahrenheit",
  pressure_unit: "kilopascal",
  speed_unit: "kilometer_per_hour",
  length_unit: "meter",
  mass_unit: "kilogram",
  date_format: "DD/MM/YYYY",
  time_format: "24h",
  week_start: "monday",
  number_format: "1,234.56",
  currency: "AUD",
  theme: "system",
};

const HELLO_CATALOG: Record<string, string> = {
  "com.nube.hello.greeting": "Hi {name}",
  "starter.settings.language.label": "Language",
};

/** 2026-04-22 02:00 UTC → 22/04/2026 12:00 Brisbane. */
const FIXTURE_TS = Date.UTC(2026, 3, 22, 2, 0, 0);

/** A panel that exercises every hook on one render. */
function HelloPanel(): React.ReactElement {
  const prefs = useHostPrefs();
  const t = useHostTranslate();
  const { formatDate, formatQuantity } = useHostFormatters();
  return (
    <div>
      <span data-testid="greeting">{t("greeting", { name: "Sam" })}</span>
      <span data-testid="platform-key">{t("starter.settings.language.label")}</span>
      <span data-testid="date">{formatDate(FIXTURE_TS)}</span>
      <span data-testid="temp">{formatQuantity(22.44, "temperature", "celsius")}</span>
      <span data-testid="language">{prefs.language}</span>
    </div>
  );
}

describe("Stage-3 prefs + i18n singleton hook surface", () => {
  it("MockHostProvider drives every hook end-to-end (en-AU + BBQ °F)", () => {
    render(
      <MockHostProvider
        extensionId="com.nube.hello"
        prefs={AU_PREFS}
        catalogs={HELLO_CATALOG}
      >
        <HelloPanel />
      </MockHostProvider>,
    );

    expect(screen.getByTestId("greeting").textContent).toBe("Hi Sam");
    expect(screen.getByTestId("platform-key").textContent).toBe("Language");
    expect(screen.getByTestId("date").textContent).toBe("22/04/2026");
    expect(screen.getByTestId("temp").textContent).toBe("72.39 °F");
    expect(screen.getByTestId("language").textContent).toBe("en-AU");
  });

  it("useHostTranslate falls back to the id when the catalog has no entry", () => {
    function MissingPanel(): React.ReactElement {
      const t = useHostTranslate();
      return <span data-testid="missing">{t("com.nube.hello.nope")}</span>;
    }
    render(
      <MockHostProvider extensionId="com.nube.hello" prefs={AU_PREFS} catalogs={HELLO_CATALOG}>
        <MissingPanel />
      </MockHostProvider>,
    );
    expect(screen.getByTestId("missing").textContent).toBe("com.nube.hello.nope");
  });

  it("useHostPrefs throws outside <HostBindingsProvider>", () => {
    // React logs the boundary error; silence it for this assertion
    // so the test output stays readable.
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    function NakedPanel(): React.ReactElement {
      useHostPrefs();
      return <></>;
    }
    expect(() => render(<NakedPanel />)).toThrow(
      /no <HostBindingsProvider> in the tree/,
    );
    errorSpy.mockRestore();
  });

  it("useHostTranslate throws when the host did not register the i18n singleton", () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    // Build a MockHostProvider-shaped tree but omit the i18n
    // singleton by hand to simulate an extension whose block.yaml
    // forgot `requires: ["@nube/starter-ui-core/i18n"]`.
    const PrefsCtx = React.createContext<HostPreferencesContextValue | undefined>(undefined);
    function PartialHost({ children }: { children: React.ReactNode }) {
      const prefsValue: HostPreferencesContextValue = {
        preferences: AU_PREFS,
        isLoading: false,
        error: null,
        setPreferences: async () => {},
      };
      const bindings = {
        extensionId: "com.nube.hello",
        singletons: Object.freeze({
          [SINGLETON_UI_CORE_PREFERENCES]: PrefsCtx,
          // i18n deliberately omitted
        }) as Readonly<Record<string, unknown>>,
      };
      return (
        <PrefsCtx.Provider value={prefsValue}>
          <HostBindingsProvider bindings={bindings}>{children}</HostBindingsProvider>
        </PrefsCtx.Provider>
      );
    }
    function NoI18nPanel(): React.ReactElement {
      const t = useHostTranslate();
      return <span>{t("greeting")}</span>;
    }
    expect(() =>
      render(
        <PartialHost>
          <NoI18nPanel />
        </PartialHost>,
      ),
    ).toThrow(/did not provide the @nube\/starter-ui-core\/i18n singleton/);
    errorSpy.mockRestore();
  });

  it("registerExtensionContributions wraps components so hooks resolve without manual wiring", () => {
    // Build the smallest tree that proves: the wrapper added by
    // registerExtensionContributions feeds handle.singletons through
    // to the hook surface. We mount the *wrapped* component under
    // the host's context providers and assert it reads them.
    const PrefsCtx = React.createContext<HostPreferencesContextValue | undefined>(undefined);
    const IntlCtx = React.createContext<HostIntlContextValue | undefined>(undefined);
    const prefsValue: HostPreferencesContextValue = {
      preferences: AU_PREFS,
      isLoading: false,
      error: null,
      setPreferences: async () => {},
    };
    const intlValue: HostIntlContextValue = {
      language: "en-AU",
      manifest: { "en-AU": "0000000000000000" },
      isLoading: false,
      error: null,
      intl: {
        formatMessage(d, v) {
          if (d.id === "com.nube.hello.greeting") {
            return `Hi ${v?.name ?? ""}`;
          }
          return d.id;
        },
      },
    };

    let registered: { components: Record<string, React.ComponentType<unknown>> } | null = null;
    const handle: ExtensionRemoteHandle = {
      id: "com.nube.hello",
      singletons: Object.freeze({
        [SINGLETON_UI_CORE_PREFERENCES]: PrefsCtx,
        [SINGLETON_UI_CORE_I18N]: IntlCtx,
      }) as Readonly<Record<string, unknown>>,
      register: (c) => {
        registered = c;
      },
    };
    registerExtensionContributions(handle, {
      components: { HelloPanel },
    });
    expect(registered).not.toBeNull();
    const Wrapped = registered!.components["HelloPanel"]!;
    expect(Wrapped).not.toBe(HelloPanel); // the SDK wrapped it

    render(
      <PrefsCtx.Provider value={prefsValue}>
        <IntlCtx.Provider value={intlValue}>
          <Wrapped />
        </IntlCtx.Provider>
      </PrefsCtx.Provider>,
    );

    expect(screen.getByTestId("greeting").textContent).toBe("Hi Sam");
    expect(screen.getByTestId("date").textContent).toBe("22/04/2026");
    expect(screen.getByTestId("temp").textContent).toBe("72.39 °F");
  });
});
