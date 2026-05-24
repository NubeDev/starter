// Wire-contract tests for the `rubix.user.*` endpoint methods.
// Mirrors `alert.test.ts`. The CSRF cookie is stubbed onto
// `globalThis.document` so `readCsrfHeader()` emits an
// `X-CSRF-Token` header the test can assert against.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./user.js";

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

describe("RubixClient user endpoints", () => {
  it("userCreate() POSTs to /api/v1/tools/rubix.user.create with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.user.created" },
      user_id: "u-1",
      email: "ada@example.com",
      role: "admin",
      created_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.userCreate({ email: "ada@example.com", role: "admin" });

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/tools/rubix.user.create");
    expect(call.headers["content-type"]).toBe("application/json");
    expect(call.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(call.body!)).toEqual({ email: "ada@example.com", role: "admin" });
    expect(out).toEqual(body);
  });

  it("userDisable() POSTs to /api/v1/tools/rubix.user.disable with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.user.disabled" },
      user_id: "u-1",
      email: "ada@example.com",
      role: "admin",
      was_already_disabled: false,
      disabled_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.userDisable({ email: "ada@example.com" });

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.user.disable");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({ email: "ada@example.com" });
    expect(out).toEqual(body);
  });

  it("userList() POSTs an empty body to /api/v1/tools/rubix.user.list with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.user.listed", params: { count: 1 } },
      count: 1,
      users: [{ user_id: "u-1", email: "ada@example.com", role: "admin" }],
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.userList();

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.user.list");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({});
    expect(out).toEqual(body);
  });

  it("userCreate() throws StarterError on a 403 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Forbidden", status: 403 }),
        { status: 403, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(
      client.userCreate({ email: "x", role: "admin" }),
    ).rejects.toMatchObject({ name: "StarterError", status: 403 });
  });
});
