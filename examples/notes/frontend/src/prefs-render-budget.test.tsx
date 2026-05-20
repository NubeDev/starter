// Stage-7 cross-cut — render budget for `<PreferencesProvider>`.
//
// `examples/notes/user-pref.md` says: "a single `setPreferences` call
// must not trigger more than one render per consumer of the context.
// The `PreferencesProvider` memoises its context value on the
// resolved prefs object identity; three sibling consumers must
// render exactly twice across a language flip (initial + post-flip)."
//
// This test mounts three independent consumers under one provider,
// counts each one's render, calls `setPreferences({ language: "es" })`,
// and asserts every consumer rendered exactly twice.
//
// Catches the easy regression where someone inlines `value={{ … }}`
// into the provider's Context — that would re-create the object on
// every render and break every consumer's `React.memo` boundary.

import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { useRef, type ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StarterClient } from "@nube/starter-client-ts";
import {
  PreferencesProvider,
  usePreferences,
  type ResolvedPreferences,
} from "@nube/starter-ui-core/preferences";

afterEach(() => cleanup());

function makePrefs(over: Partial<ResolvedPreferences> = {}): ResolvedPreferences {
  return {
    timezone: "Australia/Brisbane",
    locale: "en-AU",
    language: "en",
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
    ...over,
  };
}

function makeMockServer(initial: ResolvedPreferences) {
  let prefs = initial;
  const fetchImpl = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const method = (init?.method ?? "GET").toUpperCase();
    if (url.includes("/v1/me/preferences") && method === "GET") {
      return new Response(JSON.stringify(prefs), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }
    if (url.includes("/v1/me/preferences") && method === "PATCH") {
      const patch = JSON.parse(init?.body as string);
      prefs = { ...prefs, ...patch };
      return new Response(null, { status: 204 });
    }
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
  return { fetch: fetchImpl };
}

/**
 * One sibling consumer. Uses `useRef` to count its own renders so the
 * test doesn't rely on React DevTools internals. Reads `language` off
 * the context so a setPreferences({language: "es"}) flip is the
 * only thing that should bump the count.
 */
function Consumer({ id, counts }: { id: string; counts: Record<string, number> }) {
  const renderCount = useRef(0);
  renderCount.current += 1;
  counts[id] = renderCount.current;
  const { preferences } = usePreferences();
  return (
    <div data-testid={id}>
      {preferences?.language ?? "—"}:{renderCount.current}
    </div>
  );
}

function Wrapper({ children, fetchImpl }: { children: ReactNode; fetchImpl: typeof fetch }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const client = new StarterClient({ baseUrl: "http://t", fetch: fetchImpl });
  return (
    <QueryClientProvider client={qc}>
      <PreferencesProvider client={client}>{children}</PreferencesProvider>
    </QueryClientProvider>
  );
}

function FlipButton() {
  const { setPreferences } = usePreferences();
  return (
    <button
      data-testid="flip"
      onClick={() => {
        void setPreferences({ language: "es" });
      }}
    >
      flip
    </button>
  );
}

describe("PreferencesProvider — render budget", () => {
  it("three sibling consumers re-render exactly twice across a language flip", async () => {
    const server = makeMockServer(makePrefs());
    const counts: Record<string, number> = { a: 0, b: 0, c: 0 };

    render(
      <Wrapper fetchImpl={server.fetch}>
        <Consumer id="a" counts={counts} />
        <Consumer id="b" counts={counts} />
        <Consumer id="c" counts={counts} />
        <FlipButton />
      </Wrapper>,
    );

    // First render — initial prefs resolved, every consumer shows `en`.
    await waitFor(() => expect(screen.getByTestId("a").textContent).toMatch(/^en:/));
    expect(counts.a).toBe(1);
    expect(counts.b).toBe(1);
    expect(counts.c).toBe(1);

    // Flip — one mutation, server PATCH, query refetch, single
    // context value change, single re-render per consumer.
    await act(async () => {
      screen.getByTestId("flip").click();
    });
    await waitFor(() => expect(screen.getByTestId("a").textContent).toMatch(/^es:/));

    expect(counts.a).toBe(2);
    expect(counts.b).toBe(2);
    expect(counts.c).toBe(2);
  });
});
