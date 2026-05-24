// `useRuleWrite` / `useMartCreate` / `useRetentionSet` — write hooks
// for the `rubix.clickhouse.*` tool family.
//
// All three are mutations against rubix-agent; they share a single
// `['rubix','clickhouse']` invalidation prefix so any future list /
// inspect query under that prefix re-fetches on success. CSRF wiring
// is inherited from the typed `RubixClient.*` methods.

import {
  useMutation,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
} from "@tanstack/react-query";

import type {
  ClickhouseMartCreateRequest,
  ClickhouseMartCreateResponse,
  ClickhouseRetentionSetRequest,
  ClickhouseRetentionSetResponse,
  ClickhouseRuleWriteRequest,
  ClickhouseRuleWriteResponse,
} from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const CLICKHOUSE_KEY = ["rubix", "clickhouse"] as const;

type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** Write a clickhouse projection rule. Invalidates the clickhouse prefix on success. */
export function useRuleWrite(
  options?: WriteOptions<ClickhouseRuleWriteRequest, ClickhouseRuleWriteResponse>,
): UseMutationResult<ClickhouseRuleWriteResponse, StarterError, ClickhouseRuleWriteRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<ClickhouseRuleWriteResponse, StarterError, ClickhouseRuleWriteRequest>({
    mutationFn: (request) => client.ruleWrite(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: CLICKHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Create a clickhouse mart. Invalidates the clickhouse prefix on success. */
export function useMartCreate(
  options?: WriteOptions<ClickhouseMartCreateRequest, ClickhouseMartCreateResponse>,
): UseMutationResult<ClickhouseMartCreateResponse, StarterError, ClickhouseMartCreateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<ClickhouseMartCreateResponse, StarterError, ClickhouseMartCreateRequest>({
    mutationFn: (request) => client.martCreate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: CLICKHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Set retention days on a clickhouse table. Invalidates the clickhouse prefix. */
export function useRetentionSet(
  options?: WriteOptions<ClickhouseRetentionSetRequest, ClickhouseRetentionSetResponse>,
): UseMutationResult<
  ClickhouseRetentionSetResponse,
  StarterError,
  ClickhouseRetentionSetRequest
> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<
    ClickhouseRetentionSetResponse,
    StarterError,
    ClickhouseRetentionSetRequest
  >({
    mutationFn: (request) => client.retentionSet(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: CLICKHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
