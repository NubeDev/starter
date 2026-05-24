// Wire-contract tests for the `rubix.alert.send` endpoint method.
// Mirrors `packages/starter-client-ts/src/endpoints/auth.test.ts`.

import { describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./alert.js";

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

describe("RubixClient alert endpoint", () => {
  it("send() POSTs JSON to /api/v1/tools/rubix.alert.send and returns the body", async () => {
    const body = {
      summary: { code: "rubix.alert.send.ok" },
      severity: "warn" as const,
      delivered_chars: 21,
      probed_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.send({ severity: "warn", message: "Disk 89% full on /" });

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/tools/rubix.alert.send");
    expect(call.headers["content-type"]).toBe("application/json");
    expect(JSON.parse(call.body!)).toEqual({ severity: "warn", message: "Disk 89% full on /" });
    expect(out).toEqual(body);
  });

  it("send() throws StarterError on a 403 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Forbidden", status: 403 }),
        { status: 403, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(
      client.send({ severity: "info", message: "x" }),
    ).rejects.toMatchObject({ name: "StarterError", status: 403 });
  });
});
