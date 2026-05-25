// Transport-layer tests for `fetchJson` — specifically the
// content-type guard that turns a dev-server SPA-fallback HTML
// response into a typed `StarterError` instead of a silent
// `SyntaxError` from `res.json()`.

import { describe, expect, it } from "vitest";

import { StarterClient } from "./client.js";
import { fetchJson } from "./fetch_json.js";
import { StarterError } from "../error/starter-error.js";

function clientWith(response: Response): StarterClient {
  const fake: typeof fetch = async () => response.clone();
  return new StarterClient({ baseUrl: "http://t", fetch: fake });
}

describe("fetchJson content-type guard", () => {
  it("parses application/json bodies", async () => {
    const client = clientWith(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
    const out = await fetchJson<{ ok: boolean }>(client, "/x");
    expect(out).toEqual({ ok: true });
  });

  it("accepts application/json with a charset parameter", async () => {
    const client = clientWith(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json; charset=utf-8" },
      }),
    );
    const out = await fetchJson<{ ok: boolean }>(client, "/x");
    expect(out).toEqual({ ok: true });
  });

  it("accepts vendor +json types", async () => {
    const client = clientWith(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/vnd.api+json" },
      }),
    );
    const out = await fetchJson<{ ok: boolean }>(client, "/x");
    expect(out).toEqual({ ok: true });
  });

  it("throws a typed StarterError when a 2xx body is HTML", async () => {
    const client = clientWith(
      new Response("<!DOCTYPE html><html><body>SPA shell</body></html>", {
        status: 200,
        headers: { "content-type": "text/html" },
      }),
    );
    await expect(fetchJson(client, "/api/v1/auth/me")).rejects.toMatchObject({
      name: "StarterError",
      status: 502,
      code: "invalid-response-content-type",
    });
  });

  it("throws a typed StarterError when content-type is missing", async () => {
    const client = clientWith(new Response("nope", { status: 200 }));
    await expect(fetchJson(client, "/x")).rejects.toMatchObject({
      name: "StarterError",
      status: 502,
      code: "invalid-response-content-type",
    });
  });

  it("still surfaces non-2xx as a StarterError from the body", async () => {
    const client = clientWith(
      new Response(
        JSON.stringify({ type: "about:blank", title: "Unauthorized", status: 401 }),
        { status: 401, headers: { "content-type": "application/problem+json" } },
      ),
    );
    await expect(fetchJson(client, "/x")).rejects.toMatchObject({
      name: "StarterError",
      status: 401,
    });
  });

  it("preserves the StarterError instance type for instanceof checks", async () => {
    const client = clientWith(
      new Response("html", { status: 200, headers: { "content-type": "text/html" } }),
    );
    try {
      await fetchJson(client, "/x");
      expect.fail("expected fetchJson to throw");
    } catch (err) {
      expect(err).toBeInstanceOf(StarterError);
    }
  });
});
