// Tests for the `useDiskUsage` / `useDbHealth` / `useFlowErrors`
// read hooks. Each test drives the matching `RubixClient` method
// through the shared fetch harness and asserts query state +
// dispatch URL.

import { describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import { useDiskUsage, useDbHealth, useFlowErrors } from "./system.js";
import { jsonResponse, makeHarness } from "./test-harness.js";

describe("useDiskUsage", () => {
  it("fetches and resolves with the disk-usage probe payload", async () => {
    const body = {
      summary: { code: "rubix.system.disk.ok" },
      mount: "/",
      total_bytes: 1000,
      free_bytes: 200,
      percent_used: 80,
      probed_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));

    const { result } = renderHook(() => useDiskUsage({ mount: "/" }), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(body);
    expect(calls).toHaveLength(1);
    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.system.disk");
    expect(JSON.parse(calls[0]!.body!)).toEqual({ mount: "/" });
  });

  it("surfaces errors as `isError` (no retry under the test policy)", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "boom" } }, 500));
    const { result } = renderHook(() => useDiskUsage(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

describe("useDbHealth", () => {
  it("hits rubix.system.db with the provided dsn", async () => {
    const body = {
      summary: { code: "rubix.system.db.ok" },
      dsn: "pg://x",
      reachable: true,
      used_bytes: 0,
      probed_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useDbHealth({ dsn: "pg://x" }), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.system.db");
    expect(JSON.parse(calls[0]!.body!)).toEqual({ dsn: "pg://x" });
  });
});

describe("useFlowErrors", () => {
  it("hits rubix.system.flow_errors and threads window_secs", async () => {
    const body = {
      summary: { code: "rubix.system.flow_errors.ok" },
      window_secs: 60,
      error_count: 0,
      samples: [],
      probed_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(body));
    const { result } = renderHook(() => useFlowErrors({ window_secs: 60 }), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.system.flow_errors");
    expect(JSON.parse(calls[0]!.body!)).toEqual({ window_secs: 60 });
  });
});
