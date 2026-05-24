// Test harness shared by the hook test suites. Mounts the same
// provider stack the app uses (RubixClientProvider →
// QueryClientProvider), backed by a `fetch` stub the test records
// against. Disables retries so non-2xx responses surface
// synchronously rather than spinning through the default 3-attempt
// retry policy.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement, ReactNode } from "react";

import { StarterClient } from "@nube/starter-client-ts";
import { RubixClient } from "@nube/rubix-client-ts";

import { RubixClientProvider } from "../provider/rubix-client-provider.js";

export interface FetchCall {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: string | undefined;
}

export interface Harness {
  Wrapper: (props: { children: ReactNode }) => ReactElement;
  calls: FetchCall[];
  client: RubixClient;
  queryClient: QueryClient;
}

/** Build a fetch stub that returns `responder(call)` per request. */
export function makeHarness(
  responder: (call: FetchCall) => Response,
): Harness {
  const calls: FetchCall[] = [];
  const fakeFetch: typeof fetch = async (input, init) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : (input as Request).url;
    const headers: Record<string, string> = {};
    for (const [k, v] of Object.entries(init?.headers ?? {})) headers[k] = String(v);
    const call: FetchCall = {
      url,
      method: (init?.method ?? "GET").toUpperCase(),
      headers,
      body: typeof init?.body === "string" ? init.body : undefined,
    };
    calls.push(call);
    return responder(call).clone();
  };

  const starter = new StarterClient({ baseUrl: "http://t", fetch: fakeFetch });
  const client = new RubixClient(starter);

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: 0, gcTime: 0 },
      mutations: { retry: false },
    },
  });

  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <RubixClientProvider client={client}>{children}</RubixClientProvider>
    </QueryClientProvider>
  );

  return { Wrapper, calls, client, queryClient };
}

export function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

export const CSRF = "csrf-test-token";

export function stubCsrfCookie(): void {
  // Append to the existing jsdom document.cookie store; replacing
  // `globalThis.document` wholesale breaks @testing-library/react.
  if (typeof document !== "undefined") {
    document.cookie = `starter_csrf=${CSRF}; path=/`;
  }
}

export function clearCsrfCookie(): void {
  if (typeof document !== "undefined") {
    document.cookie = "starter_csrf=; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT";
  }
}
