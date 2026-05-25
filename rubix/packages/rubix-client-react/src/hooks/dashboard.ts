// `useDashboard*` — hooks for the `rubix.dashboard.*` tool family.
//
// `list` and `get` are `useQuery`s; the remaining five are mutations.
// All seven share the `/api/v1/tools/*` POST transport. Mutations
// invalidate the shared `['rubix','dashboard']` prefix on success.

import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
  type UseQueryOptions,
  type UseQueryResult,
} from "@tanstack/react-query";

import type {
  DashboardCreateRequest,
  DashboardCreateResponse,
  DashboardDeleteRequest,
  DashboardDeleteResponse,
  DashboardDuplicateRequest,
  DashboardDuplicateResponse,
  DashboardGetRequest,
  DashboardGetResponse,
  DashboardListRequest,
  DashboardListResponse,
  DashboardPageSetRequest,
  DashboardPageSetResponse,
  DashboardUpdateRequest,
  DashboardUpdateResponse,
} from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const DASHBOARD_KEY = ["rubix", "dashboard"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;
type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** List dashboards. Query key: `['rubix','dashboard','list', filter?]`. */
export function useDashboardList(
  request: DashboardListRequest = {},
  options?: ReadOptions<DashboardListResponse>,
): UseQueryResult<DashboardListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<DashboardListResponse, StarterError>({
    queryKey: [...DASHBOARD_KEY, "list", request.filter ?? null],
    queryFn: () => client.dashboardList(request),
    ...options,
  });
}

/** Get a single dashboard by `page_id`. Query key: `['rubix','dashboard','get', page_id]`. */
export function useDashboardGet(
  pageId: string,
  options?: ReadOptions<DashboardGetResponse>,
): UseQueryResult<DashboardGetResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<DashboardGetResponse, StarterError>({
    queryKey: [...DASHBOARD_KEY, "get", pageId],
    queryFn: () => client.dashboardGet({ page_id: pageId }),
    enabled: Boolean(pageId),
    ...options,
  });
}

/** Create a dashboard. Invalidates the dashboard prefix on success. */
export function useDashboardCreate(
  options?: WriteOptions<DashboardCreateRequest, DashboardCreateResponse>,
): UseMutationResult<DashboardCreateResponse, StarterError, DashboardCreateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<DashboardCreateResponse, StarterError, DashboardCreateRequest>({
    mutationFn: (request) => client.dashboardCreate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: DASHBOARD_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Update a dashboard revision. Invalidates the dashboard prefix on success. */
export function useDashboardUpdate(
  options?: WriteOptions<DashboardUpdateRequest, DashboardUpdateResponse>,
): UseMutationResult<DashboardUpdateResponse, StarterError, DashboardUpdateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<DashboardUpdateResponse, StarterError, DashboardUpdateRequest>({
    mutationFn: (request) => client.dashboardUpdate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: DASHBOARD_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Delete a dashboard. Invalidates the dashboard prefix on success. */
export function useDashboardDelete(
  options?: WriteOptions<DashboardDeleteRequest, DashboardDeleteResponse>,
): UseMutationResult<DashboardDeleteResponse, StarterError, DashboardDeleteRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<DashboardDeleteResponse, StarterError, DashboardDeleteRequest>({
    mutationFn: (request) => client.dashboardDelete(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: DASHBOARD_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Duplicate a dashboard. Invalidates the dashboard prefix on success. */
export function useDashboardDuplicate(
  options?: WriteOptions<DashboardDuplicateRequest, DashboardDuplicateResponse>,
): UseMutationResult<DashboardDuplicateResponse, StarterError, DashboardDuplicateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<DashboardDuplicateResponse, StarterError, DashboardDuplicateRequest>({
    mutationFn: (request) => client.dashboardDuplicate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: DASHBOARD_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/**
 * Mutate one runtime slot on a live dashboard page. Does NOT bump
 * a revision — `page_set` writes through the engine slot-write
 * chokepoint, not the revision store. Still invalidates the
 * dashboard prefix so any open `get` re-renders.
 */
export function useDashboardPageSet(
  options?: WriteOptions<DashboardPageSetRequest, DashboardPageSetResponse>,
): UseMutationResult<DashboardPageSetResponse, StarterError, DashboardPageSetRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<DashboardPageSetResponse, StarterError, DashboardPageSetRequest>({
    mutationFn: (request) => client.dashboardPageSet(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: DASHBOARD_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
