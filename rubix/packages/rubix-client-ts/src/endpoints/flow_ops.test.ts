// Wire-contract tests for the `rubix.flow_ops.*` endpoint methods.
// Mirrors `user.test.ts`. The CSRF cookie is stubbed onto
// `globalThis.document` so `readCsrfHeader()` emits an
// `X-CSRF-Token` header the test can assert against.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./flow_ops.js";

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

describe("RubixClient flow_ops endpoints", () => {
  it("flowDeploy() POSTs to /api/v1/tools/rubix.flow_ops.deploy with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.flow.deployed" },
      flow_id: "com.rubix.flow-programmer",
      revision_id: "rev-1",
      deployed_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.flowDeploy({
      flow_id: "com.rubix.flow-programmer",
      body_yaml: "id: com.rubix.flow-programmer\n",
    });

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/tools/rubix.flow_ops.deploy");
    expect(call.headers["content-type"]).toBe("application/json");
    expect(call.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(call.body!)).toEqual({
      flow_id: "com.rubix.flow-programmer",
      body_yaml: "id: com.rubix.flow-programmer\n",
    });
    expect(out).toEqual(body);
  });

  it("flowLint() POSTs to /api/v1/tools/rubix.flow_ops.lint with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.flow.linted" },
      errors: [],
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.flowLint({ body_yaml: "id: com.example.x\n" });

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.flow_ops.lint");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({ body_yaml: "id: com.example.x\n" });
    expect(out).toEqual(body);
  });

  it("flowList() POSTs an empty body to /api/v1/tools/rubix.flow_ops.list with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.flow.listed", params: { count: 1 } },
      count: 1,
      flows: [
        {
          flow_id: "com.rubix.flow-programmer",
          revision_id: "rev-1",
          body_yaml: "id: com.rubix.flow-programmer\n",
        },
      ],
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.flowList();

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.flow_ops.list");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({});
    expect(out).toEqual(body);
  });

  it("flowKinds() POSTs an empty body to /api/v1/tools/rubix.flow_ops.kinds with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.flow.kinds.listed", params: { count: 1 } },
      count: 1,
      kinds: [
        {
          kind_id: "starter.flow.counter",
          config_schema: { type: "object" },
          default_label: "Counter",
        },
      ],
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.flowKinds();

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.flow_ops.kinds");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({});
    expect(out).toEqual(body);
  });

  it("flowDuplicate() POSTs to /api/v1/tools/rubix.flow_ops.duplicate with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.flow.duplicated" },
      source_flow_id: "com.rubix.flow-programmer",
      target_flow_id: "com.example.flow-programmer-copy",
      revision_id: "rev-2",
      created_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.flowDuplicate({
      source_flow_id: "com.rubix.flow-programmer",
      target_flow_id: "com.example.flow-programmer-copy",
    });

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.flow_ops.duplicate");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({
      source_flow_id: "com.rubix.flow-programmer",
      target_flow_id: "com.example.flow-programmer-copy",
    });
    expect(out).toEqual(body);
  });

  it("flowDeploy() throws StarterError on a 403 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Forbidden", status: 403 }),
        { status: 403, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(
      client.flowDeploy({ flow_id: "x", body_yaml: "id: x\n" }),
    ).rejects.toMatchObject({ name: "StarterError", status: 403 });
  });
});
