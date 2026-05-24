// `useDiskUsage` / `useDbHealth` / `useFlowErrors` — read hooks for
// the `rubix.system.*` tool family exposed by rubix-agent.
//
// Each hook is a thin `useQuery` wrapper around the matching method
// on the ambient `RubixClient` (`client.disk()` / `client.db()` /
// `client.flowErrors()`), so the queries share the long-lived
// transport mounted by `RubixClientProvider`. Query keys are
// namespaced under `['rubix','system', ...]` so the rest of the app
// can invalidate them by prefix.

import { useQuery, type UseQueryOptions, type UseQueryResult } from "@tanstack/react-query";

import type {
  DiskUsageRequest,
  DiskUsageResponse,
  DbHealthRequest,
  DbHealthResponse,
  FlowErrorsRequest,
  FlowErrorsResponse,
} from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const DISK_USAGE_KEY = ["rubix", "system", "disk"] as const;
export const DB_HEALTH_KEY = ["rubix", "system", "db"] as const;
export const FLOW_ERRORS_KEY = ["rubix", "system", "flowErrors"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

/** Disk-usage probe. Re-runs whenever `request.mount` changes. */
export function useDiskUsage(
  request: DiskUsageRequest = {},
  options?: ReadOptions<DiskUsageResponse>,
): UseQueryResult<DiskUsageResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<DiskUsageResponse, StarterError>({
    queryKey: [...DISK_USAGE_KEY, request.mount ?? null],
    queryFn: () => client.disk(request),
    ...options,
  });
}

/** Database reachability + size probe. */
export function useDbHealth(
  request: DbHealthRequest = {},
  options?: ReadOptions<DbHealthResponse>,
): UseQueryResult<DbHealthResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<DbHealthResponse, StarterError>({
    queryKey: [...DB_HEALTH_KEY, request.dsn ?? null],
    queryFn: () => client.db(request),
    ...options,
  });
}

/** Recent flow-error samples within a rolling window. */
export function useFlowErrors(
  request: FlowErrorsRequest = {},
  options?: ReadOptions<FlowErrorsResponse>,
): UseQueryResult<FlowErrorsResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<FlowErrorsResponse, StarterError>({
    queryKey: [...FLOW_ERRORS_KEY, request.window_secs ?? null],
    queryFn: () => client.flowErrors(request),
    ...options,
  });
}
