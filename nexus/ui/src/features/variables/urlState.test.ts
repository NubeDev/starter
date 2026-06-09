import { describe, expect, it } from "vitest";

import {
  parseVariableParams,
  writeVariableParams,
} from "@/features/variables/urlState";

// URL round-trip for variable selections (item 8): a `?var-<name>=…` link
// must restore the same selection, multi repeats the param, and writing must
// preserve unrelated params (from/to/refresh).
describe("parseVariableParams", () => {
  it("reads a single selection", () => {
    expect(parseVariableParams(new URLSearchParams("var-region=us"))).toEqual({
      region: ["us"],
    });
  });

  it("accumulates a repeated (multi) param", () => {
    expect(
      parseVariableParams(new URLSearchParams("var-region=us&var-region=eu")),
    ).toEqual({ region: ["us", "eu"] });
  });

  it("ignores non-variable params", () => {
    expect(parseVariableParams(new URLSearchParams("from=now-6h"))).toEqual({});
  });
});

describe("writeVariableParams", () => {
  it("writes one param per value in name order", () => {
    const out = writeVariableParams(new URLSearchParams(), {
      region: ["us", "eu"],
      host: ["a"],
    });
    expect(out.getAll("var-region")).toEqual(["us", "eu"]);
    expect(out.getAll("var-host")).toEqual(["a"]);
  });

  it("preserves unrelated params and clears stale var params", () => {
    const out = writeVariableParams(
      new URLSearchParams("from=now-6h&var-old=x"),
      { region: ["us"] },
    );
    expect(out.get("from")).toBe("now-6h");
    expect(out.has("var-old")).toBe(false);
    expect(out.get("var-region")).toBe("us");
  });

  it("round-trips through parse", () => {
    const sel = { region: ["us", "eu"], host: ["h1"] };
    expect(parseVariableParams(writeVariableParams(new URLSearchParams(), sel))).toEqual(
      sel,
    );
  });
});
