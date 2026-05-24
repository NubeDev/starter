// Wire-contract tests for the /api/v1/auth/* endpoint methods. Each test
// captures the outgoing Request via a fake fetch and asserts the
// method, path, headers, and body the client would put on the wire.

import { describe, expect, it } from "vitest";

import { StarterClient } from "../client/client.js";
import "./auth.js";

interface Recorded {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string | undefined;
}

function record(response: Response): { client: StarterClient; calls: Recorded[] } {
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
  const client = new StarterClient({ baseUrl: "http://t", fetch: fake });
  return { client, calls };
}

describe("StarterClient auth endpoints", () => {
  it("login() POSTs JSON to /api/v1/auth/login and returns the body", async () => {
    const res = new Response(JSON.stringify({ csrf_token: "abc" }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
    const { client, calls } = record(res);

    const out = await client.login({ email: "a@b", password: "pw" });

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/auth/login");
    expect(call.headers["content-type"]).toBe("application/json");
    expect(JSON.parse(call.body!)).toEqual({ email: "a@b", password: "pw" });
    expect(out).toEqual({ csrf_token: "abc" });
  });

  it("logout() POSTs to /api/v1/auth/logout", async () => {
    const { client, calls } = record(new Response(null, { status: 204 }));
    await client.logout();
    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toBe("http://t/api/v1/auth/logout");
  });

  it("me() GETs /api/v1/auth/me and returns the body", async () => {
    const me = { subject: "u-1", email: "a@b", role: "admin" };
    const { client, calls } = record(
      new Response(JSON.stringify(me), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );

    const out = await client.me();
    expect(calls[0]!.method).toBe("GET");
    expect(calls[0]!.url).toBe("http://t/api/v1/auth/me");
    expect(out).toEqual(me);
  });

  it("login() throws StarterError on a 401 problem body", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Unauthorized", status: 401 }),
        { status: 401, headers: { "content-type": "application/problem+json" } },
      ),
    );

    await expect(client.login({ email: "x", password: "y" })).rejects.toMatchObject({
      name: "StarterError",
      status: 401,
    });
  });

  it("trims a trailing slash on baseUrl so paths join cleanly", async () => {
    const calls: Recorded[] = [];
    const fake: typeof fetch = async (input, init) => {
      const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
      calls.push({ method: init?.method ?? "GET", url, headers: {}, body: undefined });
      return new Response(JSON.stringify({ subject: "u", email: "a@b", role: "admin" }), { status: 200 });
    };
    const client = new StarterClient({ baseUrl: "http://t/", fetch: fake });
    await client.me();
    expect(calls[0]!.url).toBe("http://t/api/v1/auth/me");
  });
});
