import { describe, expect, it } from "vitest";

import { STARTER_QUERY_PREFIX, isStarterQueryKey, starterQueryKey } from "./index.js";

describe("starterQueryKey", () => {
  it("prefixes every key under 'starter'", () => {
    expect(starterQueryKey("auth", "me")).toEqual([STARTER_QUERY_PREFIX, "auth", "me"]);
  });

  it("accepts numeric segments", () => {
    expect(starterQueryKey("user", 42)).toEqual(["starter", "user", 42]);
  });
});

describe("isStarterQueryKey", () => {
  it("matches starter-owned keys", () => {
    expect(isStarterQueryKey(starterQueryKey("auth", "me"))).toBe(true);
  });

  it("rejects keys that don't start with the prefix", () => {
    expect(isStarterQueryKey(["consumer", "thing"])).toBe(false);
    expect(isStarterQueryKey([])).toBe(false);
  });
});
