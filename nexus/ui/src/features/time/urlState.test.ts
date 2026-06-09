import { describe, expect, it } from "vitest";

import { parseTimeParams, writeTimeParams } from "@/features/time/urlState";

// URL round-trip for shareable deep links (C3): a link's `?from=&to=&refresh=`
// must restore the same state, and serialising state back must produce the
// same params (no churn that would spam history).
describe("parseTimeParams", () => {
  it("reads from/to/refresh", () => {
    const p = parseTimeParams(
      new URLSearchParams("from=now-6h&to=now&refresh=10"),
    );
    expect(p.range).toEqual({ from: "now-6h", to: "now" });
    expect(p.refresh).toBe(10);
  });

  it("omits the range unless both bounds are present", () => {
    expect(parseTimeParams(new URLSearchParams("from=now-6h")).range).toBeUndefined();
    expect(parseTimeParams(new URLSearchParams("to=now")).range).toBeUndefined();
  });

  it("ignores a negative or non-numeric refresh", () => {
    expect(parseTimeParams(new URLSearchParams("refresh=-5")).refresh).toBeUndefined();
    expect(parseTimeParams(new URLSearchParams("refresh=fast")).refresh).toBeUndefined();
  });
});

describe("writeTimeParams", () => {
  it("writes from/to and refresh when on", () => {
    const out = writeTimeParams(new URLSearchParams(), {
      range: { from: "now-1h", to: "now" },
      refresh: 30,
    });
    expect(out.get("from")).toBe("now-1h");
    expect(out.get("to")).toBe("now");
    expect(out.get("refresh")).toBe("30");
  });

  it("drops refresh when off", () => {
    const out = writeTimeParams(new URLSearchParams("refresh=10"), {
      range: { from: "now-1h", to: "now" },
      refresh: 0,
    });
    expect(out.has("refresh")).toBe(false);
  });

  it("round-trips through parse", () => {
    const state = { range: { from: "now-7d", to: "now" }, refresh: 60 };
    const written = writeTimeParams(new URLSearchParams(), state);
    const parsed = parseTimeParams(written);
    expect(parsed.range).toEqual(state.range);
    expect(parsed.refresh).toBe(state.refresh);
  });

  it("preserves unrelated params", () => {
    const out = writeTimeParams(new URLSearchParams("var-region=Site-A"), {
      range: { from: "now-1h", to: "now" },
      refresh: 0,
    });
    expect(out.get("var-region")).toBe("Site-A");
  });
});
