// Tests for the extension admin hooks — list + lifecycle
// mutations. Each mutation must POST to the matching action route
// with the CSRF header and invalidate the `['rubix','extensions']`
// prefix on success.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import {
  EXTENSIONS_KEY,
  useExtensionDisable,
  useExtensionEnable,
  useExtensionRestart,
  useExtensionStart,
  useExtensionStop,
  useExtensionsList,
} from "./extensions.js";
import {
  clearCsrfCookie,
  jsonResponse,
  makeHarness,
  stubCsrfCookie,
} from "./test-harness.js";

beforeEach(stubCsrfCookie);
afterEach(clearCsrfCookie);

const listBody = {
  extensions: [
    { id: "ext-a", name: "A", enabled: true, state: "running" as const },
  ],
};

describe("useExtensionsList", () => {
  it("GETs /api/v1/extensions", async () => {
    const { Wrapper, calls } = makeHarness(() => jsonResponse(listBody));
    const { result } = renderHook(() => useExtensionsList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(calls[0]!.url).toContain("/api/v1/extensions");
    expect(calls[0]!.method).toBe("GET");
  });

  it("surfaces errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({}, 500));
    const { result } = renderHook(() => useExtensionsList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

const actionPairs: Array<[
  string,
  () => ReturnType<typeof useExtensionStart>,
]> = [
  ["start", useExtensionStart as unknown as () => ReturnType<typeof useExtensionStart>],
  ["stop", useExtensionStop as unknown as () => ReturnType<typeof useExtensionStart>],
  ["restart", useExtensionRestart as unknown as () => ReturnType<typeof useExtensionStart>],
  ["enable", useExtensionEnable as unknown as () => ReturnType<typeof useExtensionStart>],
  ["disable", useExtensionDisable as unknown as () => ReturnType<typeof useExtensionStart>],
];

describe.each(actionPairs)("useExtension%s", (action, hook) => {
  it(`POSTs to /api/v1/extensions/{id}/${action} with the CSRF header and invalidates the list`, async () => {
    const { Wrapper, calls, queryClient } = makeHarness(() => jsonResponse({}));
    queryClient.setQueryData([...EXTENSIONS_KEY, "list"], listBody);

    const { result } = renderHook(() => hook(), { wrapper: Wrapper });
    await result.current.mutateAsync({ id: "ext-a" });

    expect(calls[0]!.url).toContain(`/api/v1/extensions/ext-a/${action}`);
    expect(calls[0]!.method).toBe("POST");
    expect(calls[0]!.headers["X-CSRF-Token"]).toBeDefined();

    await waitFor(() =>
      expect(
        queryClient.getQueryState([...EXTENSIONS_KEY, "list"])?.isInvalidated,
      ).toBe(true),
    );
  });

  it("propagates errors", async () => {
    const { Wrapper } = makeHarness(() => jsonResponse({}, 500));
    const { result } = renderHook(() => hook(), { wrapper: Wrapper });
    await expect(result.current.mutateAsync({ id: "ext-a" })).rejects.toBeDefined();
  });
});
