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

  it("falls back to a generic message when the body isn't a Problem", async () => {
    const res = new Response("plain text", { status: 500 });
    const err = await StarterError.fromResponse(res);
    expect(err.status).toBe(500);
    expect(err.message).toBe("HTTP 500");
    expect(err.problem).toBeUndefined();
  });

  it("falls back when JSON parses but lacks Problem fields", async () => {
    const res = new Response(JSON.stringify({ msg: "nope" }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
    const err = await StarterError.fromResponse(res);
    expect(err.problem).toBeUndefined();
    expect(err.message).toBe("HTTP 400");
  });
});
