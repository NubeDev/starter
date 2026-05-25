// Tests for the flow_ops hooks. Covers read (list), each mutation's
// dispatch + CSRF, error propagation, and the shared
// `['rubix','flow_ops']` invalidation contract.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import {
  FLOW_OPS_KEY,
  useFlowDeploy,
  useFlowDuplicate,
  useFlowKinds,
  useFlowLint,
  useFlowList,
} from "./flow-ops.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

const listBody = {
  summary: { code: "rubix.flow_ops.listed" },
  count: 1,
  flows: [{ flow_id: "f1", revision_id: "r1", body_yaml: "id: f1\n" }],
};

const kindsBody = {
  summary: { code: "rubix.flow.kinds.listed" },
  count: 1,
  kinds: [
    {
      kind_id: "starter.flow.counter",
      config_schema: { type: "object" },
      default_label: "Counter",
    },
  ],
};

describe("useFlowList", () => {
  it("fetches and resolves", async () => {
    const { Wrapper, calls } = makeHarness(() => jsonResponse(listBody));
    const { result } = renderHook(() => useFlowList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.flow_ops.list");
  });

  it("surfaces errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useFlowList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

describe("useFlowKinds", () => {
  it("fetches and caches under ['rubix','flow_ops','kinds']", async () => {
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(kindsBody));
    const { result } = renderHook(() => useFlowKinds(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.flow_ops.kinds");
    const cached = queryClient.getQueryData([...FLOW_OPS_KEY, "kinds"]);
    expect(cached).toEqual(kindsBody);
  });
});

describe("useFlowLint", () => {
  it("dispatches to rubix.flow_ops.lint and does not invalidate", async () => {
    const body = { summary: { code: "rubix.flow_ops.linted" }, errors: [] };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(body));
    queryClient.setQueryData([...FLOW_OPS_KEY, "list"], listBody);
    const { result } = renderHook(() => useFlowLint(), { wrapper: Wrapper });
    await result.current.mutateAsync({ body_yaml: "..." });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.flow_ops.lint");
    expect(queryClient.getQueryState([...FLOW_OPS_KEY, "list"])?.isInvalidated).toBe(false);
  });
});

describe("useFlowDeploy", () => {
  it("dispatches to rubix.flow_ops.deploy and invalidates the prefix", async () => {
    const body = {
      summary: { code: "rubix.flow_ops.deployed" },
      flow_id: "f1",
      revision_id: "r2",
      deployed_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(body));
    queryClient.setQueryData([...FLOW_OPS_KEY, "list"], listBody);
    const { result } = renderHook(() => useFlowDeploy(), { wrapper: Wrapper });
    await result.current.mutateAsync({ flow_id: "f1", body_yaml: "..." });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.flow_ops.deploy");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();
    expect(queryClient.getQueryState([...FLOW_OPS_KEY, "list"])?.isInvalidated).toBe(true);
  });

  it("propagates errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({ summary: { code: "x" } }, 500));
    const { result } = renderHook(() => useFlowDeploy(), { wrapper: Wrapper });
    await expect(
      result.current.mutateAsync({ flow_id: "f1", body_yaml: "..." }),
    ).rejects.toBeDefined();
  });
});

describe("useFlowDuplicate", () => {
  it("dispatches to rubix.flow_ops.duplicate and invalidates the prefix", async () => {
    const body = {
      summary: { code: "rubix.flow_ops.duplicated" },
      source_flow_id: "f1",
      target_flow_id: "f2",
      revision_id: "r1",
      created_at_ms: 1,
    };
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse(body));
    queryClient.setQueryData([...FLOW_OPS_KEY, "list"], listBody);
    const { result } = renderHook(() => useFlowDuplicate(), { wrapper: Wrapper });
    await result.current.mutateAsync({ source_flow_id: "f1", target_flow_id: "f2" });
    expect(calls[0]!.url).toContain("/api/v1/tools/rubix.flow_ops.duplicate");
    expect(queryClient.getQueryState([...FLOW_OPS_KEY, "list"])?.isInvalidated).toBe(true);
  });
});
