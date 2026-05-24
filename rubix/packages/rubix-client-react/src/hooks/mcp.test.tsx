// Tests for `useToolsList` — happy path + error.

import { describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import { useToolsList } from "./mcp.js";
import { jsonResponse, makeHarness } from "./test-harness.js";

describe("useToolsList", () => {
  it("returns the tool catalogue", async () => {
    const result_payload = {
      tools: [
        { name: "rubix.system.disk", description: "", inputSchema: {} },
        { name: "rubix.user.list", description: "", inputSchema: {} },
      ],
    };
    const { Wrapper, calls } = makeHarness(() =>
      jsonResponse({ jsonrpc: "2.0", id: 1, result: result_payload }),
    );
    const { result } = renderHook(() => useToolsList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.tools).toHaveLength(2);
    expect(calls[0]!.url).toContain("/api/v1/mcp");
    expect(JSON.parse(calls[0]!.body!).method).toBe("tools/list");
  });

  it("includes _meta.acceptLanguage when passed", async () => {
    const { Wrapper, calls } = makeHarness(() =>
      jsonResponse({ jsonrpc: "2.0", id: 1, result: { tools: [] } }),
    );
    renderHook(() => useToolsList("es-AR"), { wrapper: Wrapper });
    await waitFor(() => expect(calls.length).toBe(1));
    expect(JSON.parse(calls[0]!.body!).params._meta.acceptLanguage).toBe("es-AR");
  });

  it("surfaces JSON-RPC errors as `isError`", async () => {
    const { Wrapper } = makeHarness(() =>
      jsonResponse({
        jsonrpc: "2.0",
        id: 1,
        error: { code: -32601, message: "method not found" },
      }),
    );
    const { result } = renderHook(() => useToolsList(), { wrapper: Wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});
