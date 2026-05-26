// Wire-contract tests for the `rubix.warehouse.*` endpoint methods.
// Mirrors `user.test.ts`. The CSRF cookie is stubbed onto
// `globalThis.document` so `readCsrfHeader()` emits an
// `X-CSRF-Token` header the test can assert against.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./warehouse.js";

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

describe("RubixClient clickhouse endpoints", () => {
  it("ruleWrite() POSTs to /api/v1/tools/rubix.warehouse.rule.write with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.warehouse.rule.written" },
      rule_name: "system_disk_rollup_1h",
      written_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.ruleWrite({
      rule_name: "system_disk_rollup_1h",
      ddl: "CREATE MATERIALIZED VIEW ...",
    });

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/tools/rubix.warehouse.rule.write");
    expect(call.headers["content-type"]).toBe("application/json");
    expect(call.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(call.body!)).toEqual({
      rule_name: "system_disk_rollup_1h",
      ddl: "CREATE MATERIALIZED VIEW ...",
    });
    expect(out).toEqual(body);
  });

  it("martCreate() POSTs to /api/v1/tools/rubix.warehouse.mart.create with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.warehouse.mart.created" },
      mart_name: "system_disk_history",
      was_already_present: false,
      created_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.martCreate({
      mart_name: "system_disk_history",
      ddl: "CREATE TABLE system_disk_history (...) ENGINE = MergeTree ORDER BY ts",
    });

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.warehouse.mart.create");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({
      mart_name: "system_disk_history",
      ddl: "CREATE TABLE system_disk_history (...) ENGINE = MergeTree ORDER BY ts",
    });
    expect(out).toEqual(body);
  });

  it("retentionSet() POSTs to /api/v1/tools/rubix.warehouse.retention.set with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.warehouse.retention.set" },
      table_name: "system_disk_history",
      prior_days: 90,
      days: 30,
      was_unchanged: false,
      set_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.retentionSet({ table_name: "system_disk_history", days: 30 });

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.warehouse.retention.set");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({ table_name: "system_disk_history", days: 30 });
    expect(out).toEqual(body);
  });

  it("ruleWrite() throws StarterError on a 403 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Forbidden", status: 403 }),
        { status: 403, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(
      client.ruleWrite({ rule_name: "x", ddl: "CREATE ..." }),
    ).rejects.toMatchObject({ name: "StarterError", status: 403 });
  });
});
