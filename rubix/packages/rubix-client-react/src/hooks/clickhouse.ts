// `useRuleWrite` / `useMartCreate` / `useRetentionSet` — write hooks
// for the `rubix.clickhouse.*` tool family.
//
// Three write verbs (`rule.write`, `mart.create`, `retention.set`)
// are dispatched through the typed `RubixClient.*` methods. Four
// read/admin verbs (`rule.list`, `mart.list`, `mart.drop`,
// `tables.list`) are dispatched directly through `fetchJson` against
// the generic `POST /api/v1/tools/{tool_id}` route because the
// backing tool ids do not yet exist in `@nube/rubix-client-ts` (the
// agent-side endpoints are still being landed — see the stage 9
// BLOCKED handover). This mirrors the inlined pattern used by
// `useAudit` for the same reason: the hook shape stays stable so the
// move to a typed `client.*` call is a one-line swap.
//
// All hooks share the `['rubix','clickhouse']` query-key prefix so
// any mutation invalidates the family. CSRF wiring is inherited
// from the typed `RubixClient.*` methods for the write verbs and
// from `readCsrfHeader()` for the inlined dispatches.

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
  ClickhouseMartCreateRequest,
  ClickhouseMartCreateResponse,
  ClickhouseRetentionSetRequest,
  ClickhouseRetentionSetResponse,
  ClickhouseRuleWriteRequest,
  ClickhouseRuleWriteResponse,
} from "@nube/rubix-client-ts";
import {
  fetchJson,
  readCsrfHeader,
  type StarterClient,
  type StarterError,
} from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const CLICKHOUSE_KEY = ["rubix", "clickhouse"] as const;

type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;
type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

/** Loose mirror of `rubix_spi::dto::clickhouse::rule_list::RuleSummary`. */
export interface ClickhouseRuleSummary {
  rule_name: string;
  ddl?: string;
  written_at_ms?: number;
}
export interface ClickhouseRulesListResponse {
  rules: ClickhouseRuleSummary[];
}

/** Loose mirror of `rubix_spi::dto::clickhouse::mart_list::MartSummary`. */
export interface ClickhouseMartSummary {
  mart_name: string;
  ddl?: string;
  created_at_ms?: number;
}
export interface ClickhouseMartsListResponse {
  marts: ClickhouseMartSummary[];
}

/** Loose mirror of `rubix_spi::dto::clickhouse::tables_list::TableSummary`. */
export interface ClickhouseTableSummary {
  table_name: string;
  engine?: string;
  retention_days?: number;
  row_count?: number;
}
export interface ClickhouseTablesListResponse {
  tables: ClickhouseTableSummary[];
}

export interface ClickhouseMartDropRequest {
  mart_name: string;
}
export interface ClickhouseMartDropResponse {
  summary: { code: string };
  mart_name: string;
  was_present: boolean;
  dropped_at_ms: number;
}

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

/**
 * List clickhouse projection rules.
 * Query key: `['rubix','clickhouse','rules']`.
 */
export function useClickhouseRulesList(
  options?: ReadOptions<ClickhouseRulesListResponse>,
): UseQueryResult<ClickhouseRulesListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<ClickhouseRulesListResponse, StarterError>({
    queryKey: [...CLICKHOUSE_KEY, "rules"],
    queryFn: () =>
      dispatchTool<ClickhouseRulesListResponse>(
        client.starter,
        "rubix.clickhouse.rule.list",
        {},
      ),
    ...options,
  });
}

/**
 * List clickhouse marts.
 * Query key: `['rubix','clickhouse','marts']`.
 */
export function useClickhouseMartsList(
  options?: ReadOptions<ClickhouseMartsListResponse>,
): UseQueryResult<ClickhouseMartsListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<ClickhouseMartsListResponse, StarterError>({
    queryKey: [...CLICKHOUSE_KEY, "marts"],
    queryFn: () =>
      dispatchTool<ClickhouseMartsListResponse>(
        client.starter,
        "rubix.clickhouse.mart.list",
        {},
      ),
    ...options,
  });
}

/**
 * Drop a clickhouse mart. Invalidates the clickhouse prefix on
 * success so the marts list and any dependent admin queries refresh.
 */
export function useClickhouseMartDrop(
  options?: WriteOptions<ClickhouseMartDropRequest, ClickhouseMartDropResponse>,
): UseMutationResult<ClickhouseMartDropResponse, StarterError, ClickhouseMartDropRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<ClickhouseMartDropResponse, StarterError, ClickhouseMartDropRequest>({
    mutationFn: (request) =>
      dispatchTool<ClickhouseMartDropResponse>(
        client.starter,
        "rubix.clickhouse.mart.drop",
        request,
      ),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: CLICKHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/**
 * List all clickhouse tables (engine, retention TTL, row count).
 * Query key: `['rubix','clickhouse','tables']`.
 */
export function useClickhouseTablesList(
  options?: ReadOptions<ClickhouseTablesListResponse>,
): UseQueryResult<ClickhouseTablesListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<ClickhouseTablesListResponse, StarterError>({
    queryKey: [...CLICKHOUSE_KEY, "tables"],
    queryFn: () =>
      dispatchTool<ClickhouseTablesListResponse>(
        client.starter,
        "rubix.clickhouse.tables.list",
        {},
      ),
    ...options,
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
