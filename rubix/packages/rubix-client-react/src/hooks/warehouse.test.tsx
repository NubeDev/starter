// Tests for the clickhouse write hooks. Covers happy-path dispatch
// to each tool id, CSRF wiring, and the shared
// `['rubix','warehouse']` invalidation contract.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook } from "@testing-library/react";

import { waitFor } from "@testing-library/react";

import {
  WAREHOUSE_KEY,
  useWarehouseMartDrop,
  useWarehouseMartsList,
  useWarehouseRulesList,
  useWarehouseTablesList,
  useMartCreate,
  useRetentionSet,
  useRuleWrite,
} from "./warehouse.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

describe("useRuleWrite", () => {
  it("dispatches and invalidates the warehouse prefix", async () => {
    const body = {
      summary: { code: "rubix.warehouse.rule.written" },
      rule_name: "r1",
      written_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(body));
    queryClient.setQueryData([...WAREHOUSE_KEY, "rules"], { rules: [] });

    const { result } = renderHook(() => useRuleWrite(), { wrapper: Wrapper });
    await result.current.mutateAsync({ rule_name: "r1", ddl: "..." });

    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.warehouse.rule.write");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
    expect(queryClient.getQueryState([...WAREHOUSE_KEY, "rules"])?.isInvalidated).toBe(true);
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
  it("dispatches to rubix.warehouse.mart.create", async () => {
    const body = {
      summary: { code: "rubix.warehouse.mart.created" },
      mart_name: "m1",
      was_already_present: false,
      created_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useMartCreate(), { wrapper: Wrapper });
    await result.current.mutateAsync({ mart_name: "m1", ddl: "..." });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.warehouse.mart.create");
  });
});

describe("useWarehouseRulesList", () => {
  it("dispatches GET-style to rule.list and caches under ['rubix','warehouse','rules']", async () => {
    const body = { rules: [{ rule_name: "r1" }] };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useWarehouseRulesList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual(body);
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.warehouse.rule.list");
    expect(calls[0]!.method).toBe("POST");
  });
});

describe("useWarehouseMartsList", () => {
  it("dispatches to mart.list", async () => {
    const body = { marts: [{ mart_name: "m1" }] };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useWarehouseMartsList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.warehouse.mart.list");
  });
});

describe("useWarehouseTablesList", () => {
  it("dispatches to tables.list", async () => {
    const body = { tables: [{ table_name: "t1" }] };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useWarehouseTablesList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.warehouse.tables.list");
  });
});

describe("useWarehouseMartDrop", () => {
  it("dispatches to mart.drop, threads CSRF, invalidates the warehouse prefix", async () => {
    const body = {
      summary: { code: "rubix.warehouse.mart.dropped" },
      mart_name: "m1",
      was_present: true,
      dropped_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(body));
    queryClient.setQueryData([...WAREHOUSE_KEY, "marts"], { marts: [] });
    const { result } = renderHook(() => useWarehouseMartDrop(), { wrapper: Wrapper });
    await result.current.mutateAsync({ mart_name: "m1" });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.warehouse.mart.drop");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
    expect(queryClient.getQueryState([...WAREHOUSE_KEY, "marts"])?.isInvalidated).toBe(true);
  });
});

describe("useRetentionSet", () => {
  it("dispatches to rubix.warehouse.retention.set", async () => {
    const body = {
      summary: { code: "rubix.warehouse.retention.set" },
      table_name: "t",
      days: 30,
      was_unchanged: false,
      set_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useRetentionSet(), { wrapper: Wrapper });
    await result.current.mutateAsync({ table_name: "t", days: 30 });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.warehouse.retention.set");
  });
});
