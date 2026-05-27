// SettingsPage happy-path: a user changes the temperature unit and
// submits; the form PATCHes `/v1/me/preferences` with the diff and
// a success toast fires.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { JSX, ReactNode } from "react";
import { StarterClient } from "@nube/starter-client-ts";

import { PreferencesProvider } from "./provider.js";
import { SettingsPage } from "./SettingsPage.js";
import type { ResolvedPreferences } from "./types.js";
import { IntlProvider, _resetManifestCacheForTesting } from "../i18n/provider.js";
import { _resetCatalogCacheForTesting } from "../i18n/fetcher.js";

afterEach(() => {
  cleanup();
  _resetManifestCacheForTesting();
  _resetCatalogCacheForTesting();
});

function makePrefs(over: Partial<ResolvedPreferences> = {}): ResolvedPreferences {
  return {
    timezone: "UTC",
    locale: "en-US",
    language: "en",
    unit_system: "metric",
    temperature_unit: "celsius",
    pressure_unit: "kilopascal",
    speed_unit: "meter_per_second",
    length_unit: "meter",
    mass_unit: "kilogram",
    date_format: "auto",
    time_format: "auto",
    week_start: "auto",
    number_format: "auto",
    currency: "USD",
    theme: "system",
    ...over,
  };
}

function makeMockClient(initial: ResolvedPreferences) {
  const state = { prefs: initial, patches: [] as unknown[] };
  const fetchImpl = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const method = (init?.method ?? "GET").toUpperCase();
    if (url.includes("/v1/me/preferences") && method === "GET") {
      return new Response(JSON.stringify(state.prefs), { status: 200 });
    }
    if (url.includes("/v1/me/preferences") && method === "PATCH") {
      const patch = JSON.parse(init?.body as string);
      state.patches.push(patch);
      state.prefs = { ...state.prefs, ...patch };
      return new Response(null, { status: 204 });
    }
    if (url.endsWith("/v1/i18n/manifest")) {
      return new Response(JSON.stringify({ en: "0000000000000000" }), { status: 200 });
    }
    if (url.includes("/v1/i18n/catalogs/")) {
      return new Response(
        JSON.stringify({
          "starter.settings.heading": "Settings",
          "starter.settings.save": "Save",
          "starter.settings.toast.saved": "Saved",
        }),
        { status: 200 },
      );
    }
    return new Response("nope", { status: 404 });
  }) as typeof fetch;
  const client = new StarterClient({ baseUrl: "http://t", fetch: fetchImpl });
  return { client, state };
}

function wrap(client: StarterClient): (props: { children: ReactNode }) => JSX.Element {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }) => (
    <QueryClientProvider client={qc}>
      <PreferencesProvider client={client}>
        <IntlProvider client={client}>{children}</IntlProvider>
      </PreferencesProvider>
    </QueryClientProvider>
  );
}

describe("<SettingsPage />", () => {
  it("submits a PATCH with the diff and fires a success toast", async () => {
    const { client, state } = makeMockClient(makePrefs());
    const onToast = vi.fn();
    const Wrap = wrap(client);
    render(
      <Wrap>
        <SettingsPage onToast={onToast} />
      </Wrap>,
    );

    // Wait for the form to render (prefs loaded).
    await waitFor(() => expect(screen.queryByTestId("settings-form")).not.toBeNull());

    // Change temperature unit to fahrenheit.
    const tempSelect = screen.getByTestId("field-temperature_unit") as HTMLSelectElement;
    await act(async () => {
      fireEvent.change(tempSelect, { target: { value: "fahrenheit" } });
    });
    expect(tempSelect.value).toBe("fahrenheit");

    // Submit.
    await act(async () => {
      fireEvent.submit(screen.getByTestId("settings-form"));
    });

    await waitFor(() => expect(state.patches.length).toBe(1));
    expect(state.patches[0]).toEqual({ temperature_unit: "fahrenheit" });
    await waitFor(() => expect(onToast).toHaveBeenCalledWith(expect.objectContaining({ kind: "success" })));
  });
});
