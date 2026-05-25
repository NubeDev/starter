// `useToolsList` — `useQuery` wrapper around `client.mcpToolsList()`.
//
// Fetches the MCP tool catalogue from rubix-agent's JSON-RPC mount
// at `/api/v1/mcp`. The optional `acceptLanguage` argument threads
// through to `params._meta.acceptLanguage` (and contributes to the
// query key) so locale changes refetch automatically.

import {
  useMutation,
  useQuery,
  type UseMutationOptions,
  type UseMutationResult,
  type UseQueryOptions,
  type UseQueryResult,
} from "@tanstack/react-query";

import type { McpToolsListResult } from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const TOOLS_LIST_KEY = ["rubix", "mcp", "tools", "list"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

export interface UseToolCallVariables {
  name: string;
  arguments: Record<string, unknown>;
  acceptLanguage?: string;
}

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

/**
 * `useToolCall` — `useMutation` wrapper around `client.mcpToolsCall()`.
 *
 * Returns the tool's `structuredContent` (typed as `TResult`). Pass
 * `{ name, arguments, acceptLanguage? }` to `mutate`. Useful for
 * agent-rooted flows surfaced as MCP tools (e.g. dashboard-assistant).
 */
export function useToolCall<TResult = unknown>(
  options?: Omit<
    UseMutationOptions<TResult, StarterError, UseToolCallVariables>,
    "mutationFn"
  >,
): UseMutationResult<TResult, StarterError, UseToolCallVariables> {
  const client = useRubixClient();
  return useMutation<TResult, StarterError, UseToolCallVariables>({
    mutationFn: ({ name, arguments: args, acceptLanguage }) =>
      client.mcpToolsCall<TResult>(
        name,
        args,
        acceptLanguage ? { acceptLanguage } : undefined,
      ),
    ...options,
  });
}
