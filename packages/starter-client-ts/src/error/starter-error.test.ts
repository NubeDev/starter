import { describe, expect, it } from "vitest";

import { StarterError } from "./starter-error.js";

describe("StarterError.fromResponse", () => {
  it("captures status and uses Problem.title as message", async () => {
    const res = new Response(
      JSON.stringify({
        type: "about:blank",
        title: "Unauthorized",
        status: 401,
        detail: "no session",
      }),
      { status: 401, headers: { "content-type": "application/problem+json" } },
    );
    const err = await StarterError.fromResponse(res);
    expect(err).toBeInstanceOf(StarterError);
    expect(err.status).toBe(401);
    expect(err.message).toBe("Unauthorized");
    expect(err.problem?.detail).toBe("no session");
  });

  it("uses a plain-text body as the message when it isn't a Problem", async () => {
    const res = new Response("invalid flow config: missing field `uri`", {
      status: 400,
    });
    const err = await StarterError.fromResponse(res);
    expect(err.status).toBe(400);
    expect(err.message).toBe("invalid flow config: missing field `uri`");
    expect(err.problem).toBeUndefined();
  });

  it("falls back to a generic message when there is no body", async () => {
    const res = new Response(null, { status: 500 });
    const err = await StarterError.fromResponse(res);
    expect(err.status).toBe(500);
    expect(err.message).toBe("HTTP 500");
    expect(err.problem).toBeUndefined();
  });

  it("uses the raw JSON text when JSON parses but lacks Problem fields", async () => {
    const res = new Response(JSON.stringify({ msg: "nope" }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
    const err = await StarterError.fromResponse(res);
    expect(err.problem).toBeUndefined();
    // Not a Problem, so the body text is surfaced verbatim rather than hidden
    // behind an opaque "HTTP 400".
    expect(err.message).toBe('{"msg":"nope"}');
  });
});

describe("StarterError.is", () => {
  it("narrows unknown values to StarterError", () => {
    const err: unknown = new StarterError(404, "Not Found");
    expect(StarterError.is(err)).toBe(true);
    if (StarterError.is(err)) {
      // type-guard narrowing — `.status` is accessible without cast.
      expect(err.status).toBe(404);
    }
    expect(StarterError.is(new Error("plain"))).toBe(false);
    expect(StarterError.is(undefined)).toBe(false);
    expect(StarterError.is("nope")).toBe(false);
    expect(StarterError.is({ status: 404 })).toBe(false);
  });

  it("also matches on status when provided", () => {
    const err = new StarterError(401, "Unauthorized");
    expect(StarterError.is(err, 401)).toBe(true);
    expect(StarterError.is(err, 500)).toBe(false);
    expect(StarterError.is(new Error("plain"), 401)).toBe(false);
  });
});

describe("StarterError.invalidResponse", () => {
  it("tags 502 with a recognizable code so callers can branch on it", () => {
    const err = StarterError.invalidResponse("http://t/api/v1/auth/me", "text/html");
    expect(err.status).toBe(502);
    expect(err.code).toBe("invalid-response-content-type");
    expect(err.message).toContain("text/html");
    expect(err.message).toContain("/api/v1/auth/me");
  });

  it("handles a missing content-type header", () => {
    const err = StarterError.invalidResponse("http://t/x", null);
    expect(err.message).toContain("<none>");
    expect(err.code).toBe("invalid-response-content-type");
  });
});
