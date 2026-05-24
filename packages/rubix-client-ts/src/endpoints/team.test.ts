// Wire-contract tests for the `rubix.team.*` endpoint methods.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./team.js";

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

describe("RubixClient team endpoints", () => {
  it("teamCreate() POSTs to /api/v1/tools/rubix.team.create with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.team.created", params: { name: "Ops" } },
      team_id: "t-1",
      name: "Ops",
      created_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.teamCreate({ name: "Ops" });

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.team.create");
    expect(calls[0]!.headers["content-type"]).toBe("application/json");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({ name: "Ops" });
    expect(out).toEqual(body);
  });

  it("teamAssign() POSTs to /api/v1/tools/rubix.team.assign with CSRF header", async () => {
    const body = {
      summary: { code: "rubix.team.assigned" },
      team_id: "t-1",
      user_id: "u-1",
      already_member: false,
      assigned_at_ms: 1764892800000,
    };
    const { client, calls } = record(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.teamAssign({ team_id: "t-1", user_id: "u-1" });

    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/tools/rubix.team.assign");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBe(CSRF);
    expect(JSON.parse(calls[0]!.body!)).toEqual({ team_id: "t-1", user_id: "u-1" });
    expect(out).toEqual(body);
  });

  it("teamAssign() throws StarterError on a 404 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Not Found", status: 404 }),
        { status: 404, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(
      client.teamAssign({ team_id: "t-x", user_id: "u-x" }),
    ).rejects.toMatchObject({ name: "StarterError", status: 404 });
  });
});
