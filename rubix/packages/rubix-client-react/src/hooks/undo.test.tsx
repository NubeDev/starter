// Tests for `useUndoLast`. Covers dispatch with CSRF and the
// "invalidate every ['rubix', ...] query" contract.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook } from "@testing-library/react";

import { useUndoLast } from "./undo.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

describe("useUndoLast", () => {
  it("dispatches to rubix.undo.last and invalidates every ['rubix', ...] query", async () => {
    const { Wrapper, calls, queryClient } = makeHarness(() =>
      jsonResponse({ group_id: "g1" }),
    );
    queryClient.setQueryData(["rubix", "users", "list"], {});
    queryClient.setQueryData(["rubix", "teams", "list"], {});
    queryClient.setQueryData(["other", "thing"], {});

    const { result } = renderHook(() => useUndoLast(), { wrapper: Wrapper });
    const res = await result.current.mutateAsync({});

    expect(res.group_id).toBe("g1");
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.undo.last");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
    expect(queryClient.getQueryState(["rubix", "users", "list"])?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(["rubix", "teams", "list"])?.isInvalidated).toBe(true);
    // Non-rubix queries untouched.
    expect(queryClient.getQueryState(["other", "thing"])?.isInvalidated).toBe(false);
  });

  it("propagates errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useUndoLast(), { wrapper: Wrapper });
    await expect(result.current.mutateAsync({})).rejects.toBeDefined();
  });
});
