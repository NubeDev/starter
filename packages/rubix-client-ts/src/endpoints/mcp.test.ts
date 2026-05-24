// Wire-contract tests for the MCP endpoint methods.
// Mirrors the fetch-mock pattern in sibling endpoint tests.

import { describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import "./mcp.js";

interface Recorded {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string | undefined;
}

function record(response: Response): { client: RubixClient; calls: Recorded[] } {
  const calls: Recorded[] = [];
  const fake: typeof fetch = async (input, init) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const headers: Record<string, string> = {};
    for (const [k, v] of Object.entries(init?.headers ?? {})) headers[k] = String(v);
    calls.push({
      method: (init?.method ?? "GET").toUpperCase(),
      url,
      headers,
      body: typeof init?.body === "string" ? init.body : undefined,
    });
    return response.clone();
  };
  const starter = new StarterClient({ baseUrl: "http://t", fetch: fake });
  return { client: new RubixClient(starter), calls };
}

function rpcOk(result: unknown): Response {
  return new Response(JSON.stringify({ jsonrpc: "2.0", id: 1, result }), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

describe("RubixClient MCP endpoint", () => {
  it("mcpToolsList() POSTs a JSON-RPC tools/list envelope to /api/v1/mcp", async () => {
    const tools = [
      { name: "com.rubix.scheduled-system-check", description: "x", inputSchema: { type: "object" } },
    ];
    const { client, calls } = record(rpcOk({ tools }));

    const out = await client.mcpToolsList();

    expect(calls).toHaveLength(1);
    const call = calls[0]!;
    expect(call.method).toBe("POST");
    expect(call.url).toBe("http://t/api/v1/mcp");
    expect(call.headers["content-type"]).toBe("application/json");
    const env = JSON.parse(call.body!);
    expect(env.jsonrpc).toBe("2.0");
    expect(env.method).toBe("tools/list");
    expect(typeof env.id).toBe("number");
    expect(env.params).toEqual({});
    expect(out.tools).toEqual(tools);
  });

  it("mcpToolsList() with en-US threads acceptLanguage into params._meta", async () => {
    const { client, calls } = record(rpcOk({ tools: [] }));

    await client.mcpToolsList({ acceptLanguage: "en-US" });

    const env = JSON.parse(calls[0]!.body!);
    expect(env.params).toEqual({ _meta: { acceptLanguage: "en-US" } });
  });

  it("mcpToolsCall() POSTs tools/call, parses structuredContent, and threads es-AR _meta", async () => {
    const structured = { ok: true, message: "hola" };
    const { client, calls } = record(
      rpcOk({
        content: [{ type: "text", text: "{\"ok\":true,\"message\":\"hola\"}" }],
        structuredContent: structured,
      }),
    );

    const out = await client.mcpToolsCall<typeof structured>(
      "com.rubix.scheduled-system-check",
      { dry_run: true },
      { acceptLanguage: "es-AR" },
    );

    expect(calls).toHaveLength(1);
    const env = JSON.parse(calls[0]!.body!);
    expect(env.method).toBe("tools/call");
    expect(env.params).toEqual({
      name: "com.rubix.scheduled-system-check",
      arguments: { dry_run: true },
      _meta: { acceptLanguage: "es-AR" },
    });
    expect(out).toEqual(structured);
  });

  it("mcpToolsCall() throws when the JSON-RPC response carries an error", async () => {
    const { client } = record(
      new Response(
        JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          error: { code: -32601, message: "Method not found" },
        }),
        { status: 200, headers: { "content-type": "application/json" } },
      ),
    );

    await expect(client.mcpToolsCall("missing", {})).rejects.toThrow(/Method not found/);
  });
});
