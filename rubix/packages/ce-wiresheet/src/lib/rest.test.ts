import { describe, expect, it } from "vitest";
import { RestError } from "./rest";

describe("RestError.debug", () => {
  it("formats a copy-pasteable round-trip dump with request + response", () => {
    const e = new RestError(
      400,
      "target property already linked",
      "/api/v0/edge",
      "POST",
      { sourceUid: 1, targetPropUid: 2 },
      { code: "edge.linked" },
    );
    expect(e.debug).toBe(
      [
        "POST /api/v0/edge",
        "→ 400 target property already linked",
        "",
        "Request:",
        '{\n  "sourceUid": 1,\n  "targetPropUid": 2\n}',
        "",
        "Response:",
        '{\n  "code": "edge.linked"\n}',
      ].join("\n"),
    );
  });

  it("omits request/response sections when absent", () => {
    const e = new RestError(404, "not found", "/api/v0/nodes/uid/9", "GET");
    expect(e.debug).toBe("GET /api/v0/nodes/uid/9\n→ 404 not found");
  });

  it("is an Error with the message set", () => {
    const e = new RestError(500, "boom", "/x");
    expect(e).toBeInstanceOf(Error);
    expect(e.message).toBe("boom");
    expect(e.status).toBe(500);
  });
});
