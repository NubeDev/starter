// End-to-end test for <PreferencesProvider> + usePreferences().
// Stubs out /v1/me/preferences via a minimal fetch shim and asserts:
// - initial GET populates the context
// - setPreferences PATCHes the body and invalidates the cache
// - the query key matches the starter namespace

import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StarterClient } from "@nube/starter-client-ts";
import type { ReactNode } from "react";

import { PreferencesProvider, usePreferences } from "./provider.js";
import type { ResolvedPreferences } from "./types.js";
import { starterQueryKey } from "../query/index.js";

afterEach(() => cleanup());

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

interface MockServer {
  fetch: typeof fetch;
  prefs: ResolvedPreferences;
  patches: unknown[];
  getCount: number;
  patchCount: number;
}

function makeMockServer(initial: ResolvedPreferences): MockServer {
  const server: MockServer = {
    prefs: initial,
    patches: [],
    getCount: 0,
    patchCount: 0,
    fetch: undefined as unknown as typeof fetch,
  };
  server.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const method = (init?.method ?? "GET").toUpperCase();
    if (url.includes("/v1/me/preferences") && method === "GET") {
      server.getCount += 1;
      return new Response(JSON.stringify(server.prefs), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }
    if (url.includes("/v1/me/preferences") && method === "PATCH") {
      server.patchCount += 1;
      const patch = JSON.parse(init?.body as string);
      server.patches.push(patch);
      server.prefs = { ...server.prefs, ...patch };
      return new Response(null, { status: 204 });
    }
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
  return server;
}

function makeWrapper(client: StarterClient, qc: QueryClient) {
  return function W({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={qc}>
        <PreferencesProvider client={client}>{children}</PreferencesProvider>
      </QueryClientProvider>
    );
  };
}

function Probe() {
  const { preferences, isLoading, setPreferences } = usePreferences();
  return (
    <div>
      <span data-testid="loading">{isLoading ? "yes" : "no"}</span>
      <span data-testid="temp">{preferences?.temperature_unit ?? "—"}</span>
      <button
        data-testid="set-f"
        onClick={() => {
          void setPreferences({ temperature_unit: "fahrenheit" });
        }}
      >
        set
      </button>
    </div>
  );
}

describe("PreferencesProvider", () => {
  it("fetches on mount and exposes ResolvedPreferences via context", async () => {
    const server = makeMockServer(makePrefs());
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<Probe />, { wrapper: makeWrapper(client, qc) });

    await waitFor(() => expect(screen.getByTestId("temp").textContent).toBe("celsius"));
    expect(server.getCount).toBe(1);
  });

  it("setPreferences PATCHes and invalidates the cache", async () => {
    const server = makeMockServer(makePrefs());
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<Probe />, { wrapper: makeWrapper(client, qc) });

    await waitFor(() => expect(screen.getByTestId("temp").textContent).toBe("celsius"));

    await act(async () => {
      screen.getByTestId("set-f").click();
    });

    await waitFor(() => expect(screen.getByTestId("temp").textContent).toBe("fahrenheit"));
    expect(server.patchCount).toBe(1);
    expect(server.patches[0]).toEqual({ temperature_unit: "fahrenheit" });
    // GET ran a second time after the cache invalidation.
    expect(server.getCount).toBeGreaterThanOrEqual(2);
  });

  it("uses the starter-namespaced query key", () => {
    expect(starterQueryKey("preferences", "@starter/default")).toEqual([
      "starter",
      "preferences",
      "@starter/default",
    ]);
  });

  it("usePreferences throws outside the provider", () => {
    // React logs the error to the console; silence it for the assertion.
    const orig = console.error;
    console.error = () => {};
    try {
      expect(() => render(<Probe />)).toThrow(/usePreferences/);
    } finally {
      console.error = orig;
    }
  });
});
