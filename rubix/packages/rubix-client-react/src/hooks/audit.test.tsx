// Tests for `useAudit`. Covers happy-path fetch, query-string
// encoding of the filter, and error surfacing.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import { useAudit } from "./audit.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

describe("useAudit", () => {
  it("calls GET /v1/audit and resolves the page", async () => {
    const page = { changes: [{ id: "c1" }], next_cursor: "n1" };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(page));
    const { result } = renderHook(() => useAudit(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.changes).toHaveLength(1);
    expect(calls[0]!.url).toContain("/v1/audit");
    expect(calls[0]!.method).toBe("GET");
  });

  it("encodes filter params into the query string", async () => {
    const page = { changes: [] };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(page));
    const { result } = renderHook(
      () => useAudit({ resource_kind: "user", limit: 25 }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("resource_kind=user");
    expect(calls[0]!.url).toContain("limit=25");
  });

  it("surfaces errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useAudit(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});
