// `useTenantList` — read hook for the `rubix.tenant.list` tool.
//
// Thin `useQuery` wrapper around `RubixClient.tenantList`. The tool
// is read-only but goes through the same `/api/v1/tools/*` POST
// transport, so the CSRF wiring is inherited from the typed method.
// Query key: `['rubix','tenants','list']`.

import { useQuery, type UseQueryOptions, type UseQueryResult } from "@tanstack/react-query";

import type { TenantListRequest, TenantListResponse } from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const TENANTS_KEY = ["rubix", "tenants"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

/** List tenants. Query key: `['rubix','tenants','list']`. */
export function useTenantList(
  request: TenantListRequest = {},
  options?: ReadOptions<TenantListResponse>,
): UseQueryResult<TenantListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<TenantListResponse, StarterError>({
    queryKey: [...TENANTS_KEY, "list"],
    queryFn: () => client.tenantList(request),
    ...options,
  });
}
