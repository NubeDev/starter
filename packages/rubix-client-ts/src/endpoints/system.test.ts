// Wire-contract tests for the `rubix.system.*` endpoint methods.
// Mirrors `packages/starter-client-ts/src/endpoints/auth.test.ts`:
// a fake `fetch` records each outgoing request, and we assert the
// method, URL, headers, and body the client puts on the wire.

import { describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./system.js";

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

describe("RubixClient system endpoints", () => {
  it("disk() POSTs JSON to /api/v1/tools/rubix.system.disk and returns the body", async () => {
    const body = {
      summary: { code: "rubix.system.disk.ok" },
      mount: "/",
      total_bytes: 1000,
      free_bytes: 500,
      percent_used: 50,
      probed_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.disk({ mount: "/" });

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/tools/rubix.system.disk");
    expect(call.headers["content-type"]).toBe("application/json");
    expect(JSON.parse(call.body!)).toEqual({ mount: "/" });
    expect(out).toEqual(body);
  });

  it("disk() sends an empty object when called with no arguments", async () => {
    const { client, calls } = record(
      new Response(
        JSON.stringify({
          summary: { code: "rubix.system.disk.ok" },
          mount: "/",
          total_bytes: 0,
          free_bytes: 0,
          percent_used: 0,
          probed_at_ms: 0,
        }),
        { status: 200, headers: { "content-type": "application/json" } },
      ),
    );

    await client.disk();
    expect(JSON.parse(calls[0]!.body!)).toEqual({});
  });

  it("db() POSTs to /api/v1/tools/rubix.system.db", async () => {
    const body = {
      summary: { code: "rubix.system.db.ok" },
      dsn: "sqlite::memory:",
      reachable: true,
      used_bytes: 0,
      probed_at_ms: 0,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.db({});
    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.system.db");
    expect(out).toEqual(body);
  });

  it("flowErrors() POSTs to /api/v1/tools/rubix.system.flow_errors with the window", async () => {
    const body = {
      summary: { code: "rubix.system.flow_errors.ok" },
      window_secs: 3600,
      error_count: 0,
      samples: [],
      probed_at_ms: 0,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.flowErrors({ window_secs: 3600 });
    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.system.flow_errors");
    expect(JSON.parse(calls[0]!.body!)).toEqual({ window_secs: 3600 });
    expect(out).toEqual(body);
  });

  it("disk() throws StarterError on a 500 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Tool failed", status: 500 }),
        { status: 500, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(client.disk()).rejects.toMatchObject({
      name: "StarterError",
      status: 500,
    });
  });
});
