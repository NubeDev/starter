// Wire-contract tests for `rubix.undo.last`. Mirrors `user.test.ts`.
// The CSRF cookie is stubbed onto `globalThis.document` so
// `readCsrfHeader()` emits an `X-CSRF-Token` header the test can
// assert against.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./undo.js";

interface Recorded {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string | undefined;
}

function record(response: Response): { client: RubixClient; calls: Recorded[] } {
  const calls: Recorded[] = [];
  const fake: typeof fetch = async (input, init) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const headers: Record<string, string> = {};
    for (const [k, v] of Object.entries(init?.headers ?? {})) headers[k] = String(v);
    calls.push({
      method: (init?.method ?? "GET").toUpperCase(),
      url,
      headers,
      body: typeof init?.body === "string" ? init.body : undefined,
    });
    return response.clone();
  };
  const starter = new StarterClient({ baseUrl: "http://t", fetch: fake });
  return { client: new RubixClient(starter), calls };
}

const CSRF = "csrf-test-token";

beforeEach(() => {
  (globalThis as unknown as { document: { cookie: string } }).document = {
    cookie: `starter_csrf=${CSRF}`,
  };
});

afterEach(() => {
  delete (globalThis as unknown as { document?: unknown }).document;
});

describe("RubixClient undo endpoint", () => {
  it("undoLast() POSTs an empty body to /api/v1/tools/rubix.undo.last with CSRF header", async () => {
    const body = { group_id: "g-1" };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.undoLast();

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/tools/rubix.undo.last");
    expect(call.headers["content-type"]).toBe("application/json");
    expect(call.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(call.body!)).toEqual({});
    expect(out).toEqual(body);
  });

  it("undoLast() forwards a `scope` filter when supplied", async () => {
    const body = { group_id: "g-2" };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.undoLast({ scope: { resource: "flow:x" } });

    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({ scope: { resource: "flow:x" } });
    expect(out).toEqual(body);
  });

  it("undoLast() throws StarterError on a 403 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Forbidden", status: 403 }),
        { status: 403, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(client.undoLast()).rejects.toMatchObject({
      name: "StarterError",
      status: 403,
    });
  });
});
