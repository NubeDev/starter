// Tests for the clickhouse write hooks. Covers happy-path dispatch
// to each tool id, CSRF wiring, and the shared
// `['rubix','clickhouse']` invalidation contract.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook } from "@testing-library/react";

import {
  CLICKHOUSE_KEY,
  useMartCreate,
  useRetentionSet,
  useRuleWrite,
} from "./clickhouse.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

describe("useRuleWrite", () => {
  it("dispatches and invalidates the clickhouse prefix", async () => {
    const body = {
      summary: { code: "rubix.clickhouse.rule.written" },
      rule_name: "r1",
      written_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(body));
    queryClient.setQueryData([...CLICKHOUSE_KEY, "rules"], { rules: [] });

    const { result } = renderHook(() => useRuleWrite(), { wrapper: Wrapper });
    await result.current.mutateAsync({ rule_name: "r1", ddl: "..." });

    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.clickhouse.rule.write");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
    expect(queryClient.getQueryState([...CLICKHOUSE_KEY, "rules"])?.isInvalidated).toBe(true);
  });

  it("surfaces errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useRuleWrite(), { wrapper: Wrapper });
    await expect(
      result.current.mutateAsync({ rule_name: "r1", ddl: "..." }),
    ).rejects.toBeDefined();
  });
});

describe("useMartCreate", () => {
  it("dispatches to rubix.clickhouse.mart.create", async () => {
    const body = {
      summary: { code: "rubix.clickhouse.mart.created" },
      mart_name: "m1",
      was_already_present: false,
      created_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useMartCreate(), { wrapper: Wrapper });
    await result.current.mutateAsync({ mart_name: "m1", ddl: "..." });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.clickhouse.mart.create");
  });
});

describe("useRetentionSet", () => {
  it("dispatches to rubix.clickhouse.retention.set", async () => {
    const body = {
      summary: { code: "rubix.clickhouse.retention.set" },
      table_name: "t",
      days: 30,
      was_unchanged: false,
      set_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useRetentionSet(), { wrapper: Wrapper });
    await result.current.mutateAsync({ table_name: "t", days: 30 });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.clickhouse.retention.set");
  });
});
