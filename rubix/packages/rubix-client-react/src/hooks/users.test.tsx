// Tests for the user read + write hooks. Covers happy-path
// dispatch, error surfacing, and that mutations invalidate the
// `['rubix','users']` prefix so a co-mounted `useUserList`
// re-fetches.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import {
  USERS_KEY,
  useUserCreate,
  useUserDisable,
  useUserList,
} from "./users.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

const listBody = {
  summary: { code: "rubix.user.listed" },
  count: 1,
  users: [{ user_id: "u1", email: "a@b", role: "admin" }],
};

describe("useUserList", () => {
  it("fetches and resolves with the list payload", async () => {
    const { Wrapper, calls } = makeHarness(() => jsonResponse(listBody));
    const { result } = renderHook(() => useUserList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.count).toBe(1);
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.user.list");
  });

  it("surfaces errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useUserList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

describe("useUserCreate", () => {
  it("invalidates ['rubix','users'] on success", async () => {
    const created = {
      summary: { code: "rubix.user.created" },
      user_id: "u2",
      email: "c@d",
      role: "admin",
      created_at_ms: 1,
    };
    const { Wrapper, queryClient } = makeHarness((call) => {
      if (call.url.endsWith("rubix.user.list")) return jsonResponse(listBody);
      return jsonResponse(created);
    });

    // Seed the list cache so we can watch it get invalidated.
    queryClient.setQueryData([...USERS_KEY, "list"], listBody);
    expect(
      queryClient.getQueryState([...USERS_KEY, "list"])?.isInvalidated ?? false,
    ).toBe(false);

    const { result } = renderHook(() => useUserCreate(), { wrapper: Wrapper });
    await result.current.mutateAsync({ email: "c@d", role: "admin" });

    await waitFor(() =>
      expect(
        queryClient.getQueryState([...USERS_KEY, "list"])?.isInvalidated,
      ).toBe(true),
    );
  });

  it("threads the CSRF header through", async () => {
    const created = {
      summary: { code: "rubix.user.created" },
      user_id: "u2",
      email: "c@d",
      role: "admin",
      created_at_ms: 1,
    };
    const { Wrapper, calls } = makeHarness(() => jsonResponse(created));
    const { result } = renderHook(() => useUserCreate(), { wrapper: Wrapper });
    await result.current.mutateAsync({ email: "c@d", role: "admin" });
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
  });

  it("propagates errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useUserCreate(), { wrapper: Wrapper });
    await expect(
      result.current.mutateAsync({ email: "c@d", role: "admin" }),
    ).rejects.toBeDefined();
  });
});

describe("useUserDisable", () => {
  it("dispatches to rubix.user.disable and invalidates the prefix", async () => {
    const disabled = {
      summary: { code: "rubix.user.disabled" },
      user_id: "u1",
      email: "a@b",
      role: "admin",
      was_already_disabled: false,
      disabled_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(disabled));
    queryClient.setQueryData([...USERS_KEY, "list"], listBody);
    const { result } = renderHook(() => useUserDisable(), { wrapper: Wrapper });
    await result.current.mutateAsync({ user_id: "u1" });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.user.disable");
    await waitFor(() =>
      expect(
        queryClient.getQueryState([...USERS_KEY, "list"])?.isInvalidated,
      ).toBe(true),
    );
  });
});
