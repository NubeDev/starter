import { describe, expect, it } from "vitest";

import { parseKindConfig, toOptionsConfig } from "@/features/variables/config";

// The opaque `options_config` parser defaults every field so a missing or
// malformed blob degrades to an empty config of the right shape, and round-
// trips back to the same jsonb the store persists.
describe("parseKindConfig", () => {
  it("parses each kind's fields", () => {
    expect(parseKindConfig("custom", { optionsText: "a,b" })).toEqual({
      kind: "custom",
      optionsText: "a,b",
    });
    expect(parseKindConfig("query", { sql: "select 1", datasourceId: "d" })).toMatchObject({
      kind: "query",
      sql: "select 1",
      datasourceId: "d",
    });
    expect(parseKindConfig("interval", { steps: ["1m", "5m"] })).toEqual({
      kind: "interval",
      steps: ["1m", "5m"],
    });
    expect(parseKindConfig("datasource", { kindFilter: "postgres" })).toEqual({
      kind: "datasource",
      kindFilter: "postgres",
    });
    expect(parseKindConfig("context", { source: "values", key: "building" })).toEqual({
      kind: "context",
      source: "values",
      key: "building",
    });
  });

  it("defaults a missing/garbled blob to an empty config", () => {
    expect(parseKindConfig("custom", null)).toEqual({ kind: "custom", optionsText: "" });
    // A garbled context source falls back to `url` (the deep-link source).
    expect(parseKindConfig("context", { source: "bogus" })).toEqual({
      kind: "context",
      source: "url",
      key: "",
    });
    expect(parseKindConfig("interval", { steps: "oops" })).toEqual({
      kind: "interval",
      steps: [],
    });
    expect(parseKindConfig("query", undefined)).toMatchObject({
      kind: "query",
      sql: "",
      datasourceId: "",
    });
  });
});

describe("toOptionsConfig", () => {
  it("strips the discriminant kind", () => {
    expect(toOptionsConfig({ kind: "custom", optionsText: "a,b" })).toEqual({
      optionsText: "a,b",
    });
  });

  it("round-trips with parseKindConfig", () => {
    const cfg = { kind: "datasource", kindFilter: "postgres" } as const;
    const back = parseKindConfig("datasource", toOptionsConfig(cfg));
    expect(back).toEqual(cfg);
  });
});
