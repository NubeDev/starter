// `useToolsList` — `useQuery` wrapper around `client.mcpToolsList()`.
//
// Fetches the MCP tool catalogue from rubix-agent's JSON-RPC mount
// at `/api/v1/mcp`. The optional `acceptLanguage` argument threads
// through to `params._meta.acceptLanguage` (and contributes to the
// query key) so locale changes refetch automatically.

import { useQuery, type UseQueryOptions, type UseQueryResult } from "@tanstack/react-query";

import type { McpToolsListResult } from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const TOOLS_LIST_KEY = ["rubix", "mcp", "tools", "list"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

export function useToolsList(
  acceptLanguage?: string,
  options?: ReadOptions<McpToolsListResult>,
): UseQueryResult<McpToolsListResult, StarterError> {
  const client = useRubixClient();
  return useQuery<McpToolsListResult, StarterError>({
    queryKey: [...TOOLS_LIST_KEY, acceptLanguage ?? null],
    queryFn: () => client.mcpToolsList(acceptLanguage ? { acceptLanguage } : undefined),
    ...options,
  });
}
