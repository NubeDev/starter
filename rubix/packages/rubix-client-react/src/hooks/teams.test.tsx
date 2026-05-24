// Tests for the team mutation hooks. Covers happy-path dispatch
// against the right tool id, CSRF wiring, error surfacing, and the
// `['rubix','teams']` prefix invalidation contract.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook } from "@testing-library/react";

import { TEAMS_KEY, useTeamAssign, useTeamCreate } from "./teams.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

describe("useTeamCreate", () => {
  it("dispatches to rubix.team.create with CSRF and invalidates the prefix", async () => {
    const created = {
      summary: { code: "rubix.team.created" },
      team_id: "t1",
      name: "ops",
      created_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(created));
    queryClient.setQueryData([...TEAMS_KEY, "list"], { teams: [] });

    const { result } = renderHook(() => useTeamCreate(), { wrapper: Wrapper });
    const res = await result.current.mutateAsync({ name: "ops" });

    expect(res.team_id).toBe("t1");
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.team.create");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
    expect(queryClient.getQueryState([...TEAMS_KEY, "list"])?.isInvalidated).toBe(true);
  });

  it("propagates errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useTeamCreate(), { wrapper: Wrapper });
    await expect(result.current.mutateAsync({ name: "ops" })).rejects.toBeDefined();
  });
});

describe("useTeamAssign", () => {
  it("dispatches to rubix.team.assign and invalidates the prefix", async () => {
    const assigned = {
      summary: { code: "rubix.team.assigned" },
      team_id: "t1",
      user_id: "u1",
      already_member: false,
      assigned_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(assigned));
    queryClient.setQueryData([...TEAMS_KEY, "list"], { teams: [] });

    const { result } = renderHook(() => useTeamAssign(), { wrapper: Wrapper });
    await result.current.mutateAsync({ team_id: "t1", user_id: "u1" });

    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.team.assign");
    expect(queryClient.getQueryState([...TEAMS_KEY, "list"])?.isInvalidated).toBe(true);
  });
});
