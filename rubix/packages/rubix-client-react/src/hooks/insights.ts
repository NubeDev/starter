// `useInsightsRulesList` / `useInsightsRuleCreate` /
// `useInsightsRuleEnable` / `useInsightsRuleDisable` — hooks for
// the `rubix.insights.*` tool family.
//
// The agent-side endpoints are not yet typed in
// `@nube/rubix-client-ts` (the rubix-agent job that registers the
// tool ids is still landing — see stage 9 BLOCKED handover), so the
// hooks dispatch directly against the generic
// `POST /api/v1/tools/{tool_id}` route via `fetchJson`. The shape
// mirrors `useAudit` and the inlined clickhouse list/drop hooks;
// once typed `client.insights*` methods land, the swap is a
// one-line change per hook.
//
// All four hooks share the `['rubix','insights']` query-key prefix
// so any mutation invalidates the family. The list query is keyed
// `['rubix','insights','rules']`; mutations invalidate the whole
// prefix on success.

import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
  type UseQueryOptions,
  type UseQueryResult,
} from "@tanstack/react-query";

import {
  fetchJson,
  readCsrfHeader,
  type StarterClient,
  type StarterError,
} from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const INSIGHTS_KEY = ["rubix", "insights"] as const;

/** Loose mirror of the agent-side `InsightsRuleSummary` DTO. */
export interface InsightsRuleSummary {
  rule_id: string;
  name?: string;
  enabled: boolean;
  body_yaml?: string;
  updated_at_ms?: number;
}

export interface InsightsRulesListResponse {
  rules: InsightsRuleSummary[];
}

export interface InsightsRuleCreateRequest {
  rule_id: string;
  body_yaml: string;
}
export interface InsightsRuleCreateResponse {
  summary: { code: string };
  rule_id: string;
  created_at_ms: number;
}

export interface InsightsRuleToggleRequest {
  rule_id: string;
}
export interface InsightsRuleToggleResponse {
  summary: { code: string };
  rule_id: string;
  enabled: boolean;
  toggled_at_ms: number;
}

type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;
type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

function dispatchTool<TRes>(
  starter: StarterClient,
  toolId: string,
  request: unknown,
): Promise<TRes> {
  return fetchJson<TRes>(starter, `/api/v1/tools/${toolId}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request ?? {}),
  });
}

/**
 * List insights rules.
 * Query key: `['rubix','insights','rules']`.
 */
export function useInsightsRulesList(
  options?: ReadOptions<InsightsRulesListResponse>,
): UseQueryResult<InsightsRulesListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<InsightsRulesListResponse, StarterError>({
    queryKey: [...INSIGHTS_KEY, "rules"],
    queryFn: () =>
      dispatchTool<InsightsRulesListResponse>(
        client.starter,
        "rubix.insights.rule.list",
        {},
      ),
    ...options,
  });
}

/** Create an insights rule. Invalidates the insights prefix on success. */
export function useInsightsRuleCreate(
  options?: WriteOptions<InsightsRuleCreateRequest, InsightsRuleCreateResponse>,
): UseMutationResult<InsightsRuleCreateResponse, StarterError, InsightsRuleCreateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<InsightsRuleCreateResponse, StarterError, InsightsRuleCreateRequest>({
    mutationFn: (request) =>
      dispatchTool<InsightsRuleCreateResponse>(
        client.starter,
        "rubix.insights.rule.create",
        request,
      ),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: INSIGHTS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Enable an insights rule. Invalidates the insights prefix on success. */
export function useInsightsRuleEnable(
  options?: WriteOptions<InsightsRuleToggleRequest, InsightsRuleToggleResponse>,
): UseMutationResult<InsightsRuleToggleResponse, StarterError, InsightsRuleToggleRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<InsightsRuleToggleResponse, StarterError, InsightsRuleToggleRequest>({
    mutationFn: (request) =>
      dispatchTool<InsightsRuleToggleResponse>(
        client.starter,
        "rubix.insights.rule.enable",
        request,
      ),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: INSIGHTS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Disable an insights rule. Invalidates the insights prefix on success. */
export function useInsightsRuleDisable(
  options?: WriteOptions<InsightsRuleToggleRequest, InsightsRuleToggleResponse>,
): UseMutationResult<InsightsRuleToggleResponse, StarterError, InsightsRuleToggleRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<InsightsRuleToggleResponse, StarterError, InsightsRuleToggleRequest>({
    mutationFn: (request) =>
      dispatchTool<InsightsRuleToggleResponse>(
        client.starter,
        "rubix.insights.rule.disable",
        request,
      ),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: INSIGHTS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
