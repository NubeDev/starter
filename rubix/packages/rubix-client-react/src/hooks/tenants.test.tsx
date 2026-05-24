// Tests for `useTenantList`. Covers happy-path fetch + error surfacing.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import { useTenantList } from "./tenants.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

const listBody = {
  summary: { code: "rubix.tenant.listed" },
  count: 1,
  tenants: [{ tenant_id: "tn1", name: "acme", locale: "en" }],
};

describe("useTenantList", () => {
  it("fetches and resolves with the list payload", async () => {
    const { Wrapper, calls } = makeHarness(() => jsonResponse(listBody));
    const { result } = renderHook(() => useTenantList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.count).toBe(1);
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.tenant.list");
  });

  it("surfaces errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useTenantList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});
