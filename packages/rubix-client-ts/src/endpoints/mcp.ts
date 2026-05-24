// `rubix.mcp.*` client methods.
//
// rubix-agent mounts `starter-mcp`'s HTTP transport at
// `POST /api/v1/mcp` (see `rubix/crates/rubix-agent/src/main.rs` —
// the binary nests the `/mcp` route under `/api/v1`). The route
// speaks JSON-RPC 2.0; this module builds the envelope and parses
// the response per the MCP `tools/list` / `tools/call` contract.
//
// Locale negotiation: callers thread `acceptLanguage` into
// `params._meta.acceptLanguage` (the MCP convention also honoured by
// the stdio transport, see
// `crates/starter-mcp/src/server/stdio_loop.rs` and the
// `mcp_stdio_test` fixture). The HTTP transport also reads the
// `Accept-Language` header, but `_meta` is the portable contract.
//
// `tools/call` returns a `result` containing `content` (a list of
// text parts) and `structuredContent` (the tool's raw JSON output —
// see `crates/starter-mcp/src/server/dispatch.rs` line 113).

import { fetchJson } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";

/** Path mounted by rubix-agent's main.rs: `Router::nest("/api/v1", mcp.router)`. */
const MCP_PATH = "/api/v1/mcp";

/** Per-call options. `acceptLanguage` becomes `params._meta.acceptLanguage`. */
export interface McpCallOptions {
  acceptLanguage?: string;
}

/** One entry of the `tools/list` result. Mirrors starter-mcp's `tools_list`. */
export interface McpToolDefinition {
  name: string;
  description: string;
  inputSchema: unknown;
}

export interface McpToolsListResult {
  tools: McpToolDefinition[];
}

/** Raw JSON-RPC 2.0 response envelope. Either `result` or `error` is set. */
interface JsonRpcResponse<T> {
  jsonrpc: "2.0";
  id: number | string | null;
  result?: T;
  error?: { code: number; message: string; data?: unknown };
}

let nextId = 1;
function rpcEnvelope(method: string, params: Record<string, unknown>): string {
  return JSON.stringify({
    jsonrpc: "2.0",
    id: nextId++,
    method,
    params,
  });
}

function buildParams(
  base: Record<string, unknown>,
  opts: McpCallOptions | undefined,
): Record<string, unknown> {
  if (!opts?.acceptLanguage) return base;
  return { ...base, _meta: { acceptLanguage: opts.acceptLanguage } };
}

async function dispatch<T>(
  client: RubixClient,
  method: string,
  params: Record<string, unknown>,
): Promise<T> {
  const body = rpcEnvelope(method, params);
  const resp = await fetchJson<JsonRpcResponse<T>>(client.starter, MCP_PATH, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body,
  });
  if (resp.error) {
    throw new Error(`MCP ${method} failed: ${resp.error.code} ${resp.error.message}`);
  }
  if (resp.result === undefined) {
    throw new Error(`MCP ${method} returned no result`);
  }
  return resp.result;
}

declare module "../client/client.js" {
  interface RubixClient {
    /** JSON-RPC `tools/list`. Returns the registered MCP tool catalogue. */
    mcpToolsList(opts?: McpCallOptions): Promise<McpToolsListResult>;
    /**
     * JSON-RPC `tools/call`. Returns the tool's `structuredContent`
     * (the raw JSON output the tool produced — text rendering lives
     * in the sibling `content` array, ignored here).
     */
    mcpToolsCall<T = unknown>(
      name: string,
      args: Record<string, unknown>,
      opts?: McpCallOptions,
    ): Promise<T>;
  }
}

RubixClient.prototype.mcpToolsList = function mcpToolsList(
  this: RubixClient,
  opts?: McpCallOptions,
): Promise<McpToolsListResult> {
  return dispatch<McpToolsListResult>(this, "tools/list", buildParams({}, opts));
};

RubixClient.prototype.mcpToolsCall = async function mcpToolsCall<T = unknown>(
  this: RubixClient,
  name: string,
  args: Record<string, unknown>,
  opts?: McpCallOptions,
): Promise<T> {
  const result = await dispatch<{ structuredContent: T; content?: unknown }>(
    this,
    "tools/call",
    buildParams({ name, arguments: args }, opts),
  );
  return result.structuredContent;
};
