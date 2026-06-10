import { describe, expect, it } from "vitest";

import type { PageContext } from "@/data/types";
import {
  EMPTY_PAGE_CONTEXT,
  isContextToken,
  resolveContextToken,
  resolveContextValue,
} from "@/features/variables/context";

const ctx: PageContext = {
  nav: {
    nodeId: "n1",
    slug: "energy-overview",
    name: "Building-1",
    path: ["Buildings", "North"],
  },
  url: { building: "b-url", multi: ["x", "y"] },
  tags: { building: "b-tag", empty: null },
  values: { building: "b1" },
};

describe("resolveContextValue", () => {
  it("nav source reads slug / name / path[n]", () => {
    expect(resolveContextValue({ source: "nav", key: "slug" }, ctx)).toBe(
      "energy-overview",
    );
    expect(resolveContextValue({ source: "nav", key: "name" }, ctx)).toBe(
      "Building-1",
    );
    expect(resolveContextValue({ source: "nav", key: "path[1]" }, ctx)).toBe(
      "North",
    );
    expect(
      resolveContextValue({ source: "nav", key: "path[9]" }, ctx),
    ).toBeUndefined();
  });

  it("values source reads the nav node's mount overrides", () => {
    expect(resolveContextValue({ source: "values", key: "building" }, ctx)).toBe(
      "b1",
    );
  });

  it("url source reads a bare param, collapsing multi to the first", () => {
    expect(resolveContextValue({ source: "url", key: "building" }, ctx)).toBe(
      "b-url",
    );
    expect(resolveContextValue({ source: "url", key: "multi" }, ctx)).toBe("x");
  });

  it("tag source reads the dashboard's tag; a null tag is undefined", () => {
    expect(resolveContextValue({ source: "tag", key: "building" }, ctx)).toBe(
      "b-tag",
    );
    expect(
      resolveContextValue({ source: "tag", key: "empty" }, ctx),
    ).toBeUndefined();
  });

  it("an absent nav yields undefined for nav-source reads", () => {
    expect(
      resolveContextValue({ source: "nav", key: "slug" }, EMPTY_PAGE_CONTEXT),
    ).toBeUndefined();
  });
});

describe("built-in context tokens", () => {
  it("resolves $__nav_slug / $__nav_name / $__tag(key)", () => {
    expect(resolveContextToken("__nav_slug", ctx)).toBe("energy-overview");
    expect(resolveContextToken("__nav_name", ctx)).toBe("Building-1");
    expect(resolveContextToken("__tag(building)", ctx)).toBe("b-tag");
  });

  it("an unknown token or absent value is undefined", () => {
    expect(resolveContextToken("__nope", ctx)).toBeUndefined();
    expect(resolveContextToken("__tag(missing)", ctx)).toBeUndefined();
  });

  it("isContextToken recognises only the built-in shapes", () => {
    expect(isContextToken("__nav_slug")).toBe(true);
    expect(isContextToken("__tag(building)")).toBe(true);
    expect(isContextToken("__dashboard")).toBe(false);
    expect(isContextToken("region")).toBe(false);
  });
});
