// `useFlowList` / `useFlowLint` / `useFlowDeploy` / `useFlowDuplicate`
// — hooks for the `rubix.flow_ops.*` tool family.
//
// `flowList` is a `useQuery`; the rest are mutations. All four go
// through the same `/api/v1/tools/*` POST transport, so `lint` and
// `list` thread CSRF for symmetry. Mutations invalidate the shared
// `['rubix','flow_ops']` prefix on success.

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
  FlowDeployRequest,
  FlowDeployResponse,
  FlowDuplicateRequest,
  FlowDuplicateResponse,
  FlowLintRequest,
  FlowLintResponse,
  FlowListRequest,
  FlowListResponse,
} from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const FLOW_OPS_KEY = ["rubix", "flow_ops"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;
type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** List deployed flows. Query key: `['rubix','flow_ops','list']`. */
export function useFlowList(
  request: FlowListRequest = {},
  options?: ReadOptions<FlowListResponse>,
): UseQueryResult<FlowListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<FlowListResponse, StarterError>({
    queryKey: [...FLOW_OPS_KEY, "list"],
    queryFn: () => client.flowList(request),
    ...options,
  });
}

/**
 * Lint a flow body. Modelled as a mutation rather than a query because
 * the caller supplies fresh YAML per invocation; caching by YAML hash
 * would only confuse callers. Does NOT invalidate the prefix.
 */
export function useFlowLint(
  options?: WriteOptions<FlowLintRequest, FlowLintResponse>,
): UseMutationResult<FlowLintResponse, StarterError, FlowLintRequest> {
  const client = useRubixClient();
  return useMutation<FlowLintResponse, StarterError, FlowLintRequest>({
    mutationFn: (request) => client.flowLint(request),
    ...options,
  });
}

/** Deploy a flow revision. Invalidates the flow_ops prefix on success. */
export function useFlowDeploy(
  options?: WriteOptions<FlowDeployRequest, FlowDeployResponse>,
): UseMutationResult<FlowDeployResponse, StarterError, FlowDeployRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<FlowDeployResponse, StarterError, FlowDeployRequest>({
    mutationFn: (request) => client.flowDeploy(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: FLOW_OPS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Duplicate a flow to a new id. Invalidates the flow_ops prefix on success. */
export function useFlowDuplicate(
  options?: WriteOptions<FlowDuplicateRequest, FlowDuplicateResponse>,
): UseMutationResult<FlowDuplicateResponse, StarterError, FlowDuplicateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<FlowDuplicateResponse, StarterError, FlowDuplicateRequest>({
    mutationFn: (request) => client.flowDuplicate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: FLOW_OPS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
