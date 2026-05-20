// Stage-1 smoke test: the notes host's prefs + i18n wiring resolves
// an Australian operator's preferences and renders them correctly.
//
// What the test proves:
//   1. `<PrefsHostShell>` actually mounts `<PreferencesProvider>` +
//      `<IntlProvider>` in the right order and routes both at the
//      caller's `StarterClient` (stubbed fetch).
//   2. The loading contract holds — children only mount after prefs
//      resolve; while in-flight the fallback (not the probe) renders.
//   3. The en-AU + BBQ-°F + Australia/Brisbane resolved prefs produce
//      `22/04/2026` and `72.4 °F`.
//   4. `<PreferencesProvider>` writes the resolved language onto
//      `document.documentElement.lang` so a11y / hyphenation / UA
//      heuristics line up.

import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { StarterClient } from "@nube/starter-client-ts";
import type { ResolvedPreferences } from "@nube/starter-ui-core/preferences";

import { PrefsHostShell, PrefsProbe } from "./prefs-host.js";

afterEach(() => {
  cleanup();
  // Reset the side-effect target so each test starts fresh.
  document.documentElement.lang = "";
});

/** en-AU operator with a BBQ override on the temperature unit. */
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

/** Minimal fetch stub: serves `/v1/me/preferences` + an empty i18n
 * manifest. The catalog branch is reached only if `<IntlProvider>`
 * actually mounts — proving the second provider is wired too. */
function makeStubFetch(prefs: ResolvedPreferences) {
  const calls: string[] = [];
  const fetch = (async (input: RequestInfo | URL) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    calls.push(url);
    if (url.includes("/v1/me/preferences")) {
      return new Response(JSON.stringify(prefs), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }
    if (url.includes("/v1/i18n/manifest")) {
      // Empty manifest — IntlProvider falls back to `en` per R5;
      // good enough for Stage 1 because PrefsProbe formats via
      // `Intl.*` directly, not through react-intl.
      return new Response(JSON.stringify({}), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }
    if (url.includes("/v1/i18n/catalogs/")) {
      return new Response("{}", {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }
    return new Response("not found", { status: 404 });
  }) as typeof globalThis.fetch;
  return { fetch, calls };
}

describe("notes host — Stage 1 prefs + i18n wiring", () => {
  it("renders en-AU date, °F temperature, and sets <html lang>", async () => {
    const { fetch } = makeStubFetch(AU_PREFS);
    const client = new StarterClient({ baseUrl: "http://t", fetch });

    render(
      <PrefsHostShell client={client} fallback={<span>loading…</span>}>
        <PrefsProbe />
      </PrefsHostShell>,
    );

    // Before prefs resolve, the loading contract must hold: the
    // probe (a child of PreferencesProvider) is NOT in the tree.
    expect(screen.queryByTestId("prefs-probe")).toBeNull();
    expect(screen.getByText("loading…")).toBeTruthy();

    // After prefs resolve, the probe mounts and renders against the
    // resolved prefs.
    await waitFor(() => {
      expect(screen.getByTestId("prefs-probe")).toBeTruthy();
    });

    expect(screen.getByTestId("prefs-probe-date").textContent).toBe("22/04/2026");
    expect(screen.getByTestId("prefs-probe-temp").textContent).toBe("72.4 °F");

    // The provider keeps `<html lang>` in sync with prefs.language.
    await waitFor(() => {
      expect(document.documentElement.lang).toBe("en-AU");
    });
  });
});
