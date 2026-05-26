// `useRuleWrite` / `useMartCreate` / `useRetentionSet` — write hooks
// for the `rubix.warehouse.*` tool family.
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
// All hooks share the `['rubix','warehouse']` query-key prefix so
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
  WarehouseMartCreateRequest,
  WarehouseMartCreateResponse,
  WarehouseRetentionSetRequest,
  WarehouseRetentionSetResponse,
  WarehouseRuleWriteRequest,
  WarehouseRuleWriteResponse,
} from "@nube/rubix-client-ts";
import {
  fetchJson,
  readCsrfHeader,
  type StarterClient,
  type StarterError,
} from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const WAREHOUSE_KEY = ["rubix", "warehouse"] as const;

type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;
type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

/** Loose mirror of `rubix_spi::dto::warehouse::rule_list::RuleSummary`. */
export interface WarehouseRuleSummary {
  rule_name: string;
  ddl?: string;
  written_at_ms?: number;
}
export interface WarehouseRulesListResponse {
  rules: WarehouseRuleSummary[];
}

/** Loose mirror of `rubix_spi::dto::warehouse::mart_list::MartSummary`. */
export interface WarehouseMartSummary {
  mart_name: string;
  ddl?: string;
  created_at_ms?: number;
}
export interface WarehouseMartsListResponse {
  marts: WarehouseMartSummary[];
}

/** Loose mirror of `rubix_spi::dto::warehouse::tables_list::TableSummary`. */
export interface WarehouseTableSummary {
  table_name: string;
  engine?: string;
  retention_days?: number;
  row_count?: number;
}
export interface WarehouseTablesListResponse {
  tables: WarehouseTableSummary[];
}

export interface WarehouseMartDropRequest {
  mart_name: string;
}
export interface WarehouseMartDropResponse {
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

/** Write a warehouse projection rule. Invalidates the warehouse prefix on success. */
export function useRuleWrite(
  options?: WriteOptions<WarehouseRuleWriteRequest, WarehouseRuleWriteResponse>,
): UseMutationResult<WarehouseRuleWriteResponse, StarterError, WarehouseRuleWriteRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<WarehouseRuleWriteResponse, StarterError, WarehouseRuleWriteRequest>({
    mutationFn: (request) => client.ruleWrite(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: WAREHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Create a warehouse mart. Invalidates the warehouse prefix on success. */
export function useMartCreate(
  options?: WriteOptions<WarehouseMartCreateRequest, WarehouseMartCreateResponse>,
): UseMutationResult<WarehouseMartCreateResponse, StarterError, WarehouseMartCreateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<WarehouseMartCreateResponse, StarterError, WarehouseMartCreateRequest>({
    mutationFn: (request) => client.martCreate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: WAREHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/**
 * List warehouse projection rules.
 * Query key: `['rubix','warehouse','rules']`.
 */
export function useWarehouseRulesList(
  options?: ReadOptions<WarehouseRulesListResponse>,
): UseQueryResult<WarehouseRulesListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<WarehouseRulesListResponse, StarterError>({
    queryKey: [...WAREHOUSE_KEY, "rules"],
    queryFn: () =>
      dispatchTool<WarehouseRulesListResponse>(
        client.starter,
        "rubix.warehouse.rule.list",
        {},
      ),
    ...options,
  });
}

/**
 * List warehouse marts.
 * Query key: `['rubix','warehouse','marts']`.
 */
export function useWarehouseMartsList(
  options?: ReadOptions<WarehouseMartsListResponse>,
): UseQueryResult<WarehouseMartsListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<WarehouseMartsListResponse, StarterError>({
    queryKey: [...WAREHOUSE_KEY, "marts"],
    queryFn: () =>
      dispatchTool<WarehouseMartsListResponse>(
        client.starter,
        "rubix.warehouse.mart.list",
        {},
      ),
    ...options,
  });
}

/**
 * Drop a warehouse mart. Invalidates the warehouse prefix on
 * success so the marts list and any dependent admin queries refresh.
 */
export function useWarehouseMartDrop(
  options?: WriteOptions<WarehouseMartDropRequest, WarehouseMartDropResponse>,
): UseMutationResult<WarehouseMartDropResponse, StarterError, WarehouseMartDropRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<WarehouseMartDropResponse, StarterError, WarehouseMartDropRequest>({
    mutationFn: (request) =>
      dispatchTool<WarehouseMartDropResponse>(
        client.starter,
        "rubix.warehouse.mart.drop",
        request,
      ),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: WAREHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/**
 * List all warehouse tables (engine, retention TTL, row count).
 * Query key: `['rubix','warehouse','tables']`.
 */
export function useWarehouseTablesList(
  options?: ReadOptions<WarehouseTablesListResponse>,
): UseQueryResult<WarehouseTablesListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<WarehouseTablesListResponse, StarterError>({
    queryKey: [...WAREHOUSE_KEY, "tables"],
    queryFn: () =>
      dispatchTool<WarehouseTablesListResponse>(
        client.starter,
        "rubix.warehouse.tables.list",
        {},
      ),
    ...options,
  });
}

/** Set retention days on a warehouse table. Invalidates the warehouse prefix. */
export function useRetentionSet(
  options?: WriteOptions<WarehouseRetentionSetRequest, WarehouseRetentionSetResponse>,
): UseMutationResult<
  WarehouseRetentionSetResponse,
  StarterError,
  WarehouseRetentionSetRequest
> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<
    WarehouseRetentionSetResponse,
    StarterError,
    WarehouseRetentionSetRequest
  >({
    mutationFn: (request) => client.retentionSet(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: WAREHOUSE_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
