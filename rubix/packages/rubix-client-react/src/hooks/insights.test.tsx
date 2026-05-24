// Tests for the rubix.insights.* hooks. Covers happy-path tool-id
// dispatch, CSRF threading on mutations, and the shared
// `['rubix','insights']` invalidation contract.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import {
  INSIGHTS_KEY,
  useInsightsRuleCreate,
  useInsightsRuleDisable,
  useInsightsRuleEnable,
  useInsightsRulesList,
} from "./insights.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

describe("useInsightsRulesList", () => {
  it("dispatches to rule.list and caches under ['rubix','insights','rules']", async () => {
    const body = { rules: [{ rule_id: "r1", enabled: true }] };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useInsightsRulesList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual(body);
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.insights.rule.list");
  });
});

describe("useInsightsRuleCreate", () => {
  it("dispatches to rule.create, threads CSRF, invalidates the insights prefix", async () => {
    const body = {
      summary: { code: "rubix.insights.rule.created" },
      rule_id: "r1",
      created_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(body));
    queryClient.setQueryData([...INSIGHTS_KEY, "rules"], { rules: [] });

    const { result } = renderHook(() => useInsightsRuleCreate(), { wrapper: Wrapper });
    await result.current.mutateAsync({ rule_id: "r1", body_yaml: "x: 1" });

    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.insights.rule.create");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
    expect(queryClient.getQueryState([...INSIGHTS_KEY, "rules"])?.isInvalidated).toBe(true);
  });

  it("surfaces errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useInsightsRuleCreate(), { wrapper: Wrapper });
    await expect(
      result.current.mutateAsync({ rule_id: "r1", body_yaml: "" }),
    ).rejects.toBeDefined();
  });
});

describe("useInsightsRuleEnable", () => {
  it("dispatches to rule.enable", async () => {
    const body = {
      summary: { code: "rubix.insights.rule.enabled" },
      rule_id: "r1",
      enabled: true,
      toggled_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useInsightsRuleEnable(), { wrapper: Wrapper });
    await result.current.mutateAsync({ rule_id: "r1" });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.insights.rule.enable");
  });
});

describe("useInsightsRuleDisable", () => {
  it("dispatches to rule.disable", async () => {
    const body = {
      summary: { code: "rubix.insights.rule.disabled" },
      rule_id: "r1",
      enabled: false,
      toggled_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useInsightsRuleDisable(), { wrapper: Wrapper });
    await result.current.mutateAsync({ rule_id: "r1" });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.insights.rule.disable");
  });
});
