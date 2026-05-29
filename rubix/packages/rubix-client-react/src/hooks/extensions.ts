// Extension admin hooks — list + lifecycle mutations.
//
// The matching typed methods on `RubixClient` (`extensionsList`,
// `extensionsStart`, …) are scheduled to land via the
// `rubix-client-ts` extensions endpoint module. Until that ships
// these hooks talk to rubix-agent directly through `fetchJson` /
// `readCsrfHeader` on the wrapped starter client. The hook shapes
// match the eventual typed-method API so swapping the call sites is
// a single-line change per hook.

import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
  type UseQueryOptions,
  type UseQueryResult,
} from "@tanstack/react-query";

import { fetchJson, readCsrfHeader, type StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const EXTENSIONS_KEY = ["rubix", "extensions"] as const;

export interface ExtensionContributesUiExpose {
  name: string;
  module: string;
  slot: string;
}

export interface ExtensionContributesUi {
  entry: string;
  exposes?: ExtensionContributesUiExpose[];
}

/** Compact counts + UI block — see `ContributesSummary` in
 * `starter-ext-server/src/routes.rs`. Lets the list view skip a per-row
 * detail fetch. */
export interface ExtensionContributesSummary {
  tools: number;
  cli: number;
  rest: number;
  grpc: number;
  workers: number;
  nodes: number;
  skills: number;
  ui?: ExtensionContributesUi;
}

export interface ExtensionSummary {
  id: string;
  name: string;
  enabled: boolean;
  state: "running" | "stopped" | "starting" | "stopping" | "errored";
  last_error?: string | null;
  contributes?: ExtensionContributesSummary;
}

export interface ExtensionListResponse {
  extensions: ExtensionSummary[];
}

/** `POST /api/v1/extensions/{id}/{action}` — empty response on success. */
export interface ExtensionMutationVars {
  id: string;
}

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;
type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** List installed extensions. Query key: `['rubix','extensions','list']`. */
export function useExtensionsList(
  options?: ReadOptions<ExtensionListResponse>,
): UseQueryResult<ExtensionListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<ExtensionListResponse, StarterError>({
    queryKey: [...EXTENSIONS_KEY, "list"],
    queryFn: () => fetchJson<ExtensionListResponse>(client.starter, "/api/v1/extensions"),
    ...options,
  });
}

function makeAction(action: "start" | "stop" | "restart" | "enable" | "disable") {
  return function useExtensionAction(
    options?: WriteOptions<ExtensionMutationVars, void>,
  ): UseMutationResult<void, StarterError, ExtensionMutationVars> {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    const client = useRubixClient();
    // eslint-disable-next-line react-hooks/rules-of-hooks
    const qc = useQueryClient();
    // eslint-disable-next-line react-hooks/rules-of-hooks
    return useMutation<void, StarterError, ExtensionMutationVars>({
      mutationFn: async ({ id }) => {
        await fetchJson<unknown>(
          client.starter,
          `/api/v1/extensions/${encodeURIComponent(id)}/${action}`,
          {
            method: "POST",
            headers: { "content-type": "application/json", ...readCsrfHeader() },
            body: "{}",
          },
        );
      },
      ...options,
      onSuccess: async (...args) => {
        await qc.invalidateQueries({ queryKey: EXTENSIONS_KEY });
        await options?.onSuccess?.(...args);
      },
    });
  };
}

export const useExtensionStart = makeAction("start");
export const useExtensionStop = makeAction("stop");
export const useExtensionRestart = makeAction("restart");
export const useExtensionEnable = makeAction("enable");
export const useExtensionDisable = makeAction("disable");

// ---------------------------------------------------------------------------
// Comprehensive admin surface — issues / process / metrics / cleanup.
//
// These mirror the starter-ext-server endpoints (`/extensions/{id}/{issues,
// process,metrics,cleanup}` + `DELETE ?purge=true`). The wire payloads carry
// stable, non-localised codes (`ext.issue.*`, `ext.process.not_running`); the
// console maps those onto `rubix.extension.*` MessageKeys.
// ---------------------------------------------------------------------------

/** A single diagnostic from `GET /extensions/{id}/issues`. */
export interface ExtensionIssue {
  /** Stable wire code, e.g. `ext.issue.crashed`. */
  code: string;
  /** `info` | `warning` | `error` | `fatal`. */
  severity: "info" | "warning" | "error" | "fatal";
  /** Operator-facing context (not a localised key). */
  detail: string;
  /** `manifest` | `supervisor` | `worker` | `capability` | `health`. */
  source: string;
  /** Event-ring sequence when derived from an event. */
  seq?: number | null;
  /** Wall-clock timestamp (serde `SystemTime`). */
  at?: { secs_since_epoch: number; nanos_since_epoch: number };
}

export interface ExtensionIssuesResponse {
  issues: ExtensionIssue[];
}

/** Live process stats from `GET /extensions/{id}/process` (or `404`). */
export interface ExtensionProcessStats {
  pid: number;
  started_at?: { secs_since_epoch: number; nanos_since_epoch: number };
  uptime?: { secs: number; nanos: number };
  rss_bytes?: number | null;
  cpu_pct?: number | null;
  restarts: number;
}

/** Merged counters + gauges from `GET /extensions/{id}/metrics`. */
export interface ExtensionMetrics {
  process: ExtensionProcessStats | null;
  lifecycle_state: string;
  restarts_total: number;
  capability_violations_total: number;
  tool_calls_total: number;
  tool_errors_total: number;
  rest_requests_total: number;
  worker_runs_total: number;
  worker_failures_total: number;
  events_dropped_total: number;
}

/** One reclaimable resource from the cleanup dry-run / purge. */
export interface ExtensionCleanupItem {
  /** `warehouse_table` | `enablement_row` | `ui_cache` | `i18n_cache` | `skill` | `subscription`. */
  kind: string;
  label: string;
  bytes?: number | null;
}

/** What uninstall will (or did) do with the bundle directory itself.
 * `will_delete = false` signals a dev-mounted bundle whose source files
 * are preserved; the dialog swaps copy and confirm-button label
 * accordingly. */
export interface ExtensionBundleOutcome {
  /** Filesystem path of the bundle directory. */
  path: string;
  /** `true` for installed bundles (the runtime owns the dir);
   *  `false` for dev mounts (source files are user-owned). */
  will_delete: boolean;
}

/** Dry-run manifest from `GET /extensions/{id}/cleanup`. */
export interface ExtensionCleanupPreview {
  id: string;
  items: ExtensionCleanupItem[];
  total_bytes: number;
  bundle: ExtensionBundleOutcome;
}

/** Result of `DELETE /extensions/{id}?purge=true`. */
export interface ExtensionPurgeResult {
  id: string;
  code: string;
  removed: ExtensionCleanupItem[];
  bundle: ExtensionBundleOutcome;
}

/** Consolidated issues for one extension. */
export function useExtensionIssues(
  id: string,
  options?: ReadOptions<ExtensionIssuesResponse>,
): UseQueryResult<ExtensionIssuesResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<ExtensionIssuesResponse, StarterError>({
    queryKey: [...EXTENSIONS_KEY, "issues", id],
    queryFn: () =>
      fetchJson<ExtensionIssuesResponse>(
        client.starter,
        `/api/v1/extensions/${encodeURIComponent(id)}/issues`,
      ),
    ...options,
  });
}

/** Live PID + process stats. Disabled-by-default `retry` so a `404`
 * (`ext.process.not_running`, expected for builtin/wasm/stopped) does not
 * thrash — callers hide the Process tab when this errors. */
export function useExtensionProcess(
  id: string,
  options?: ReadOptions<ExtensionProcessStats>,
): UseQueryResult<ExtensionProcessStats, StarterError> {
  const client = useRubixClient();
  return useQuery<ExtensionProcessStats, StarterError>({
    queryKey: [...EXTENSIONS_KEY, "process", id],
    queryFn: () =>
      fetchJson<ExtensionProcessStats>(
        client.starter,
        `/api/v1/extensions/${encodeURIComponent(id)}/process`,
      ),
    retry: false,
    ...options,
  });
}

/** Sampled counters + gauges. */
export function useExtensionMetrics(
  id: string,
  options?: ReadOptions<ExtensionMetrics>,
): UseQueryResult<ExtensionMetrics, StarterError> {
  const client = useRubixClient();
  return useQuery<ExtensionMetrics, StarterError>({
    queryKey: [...EXTENSIONS_KEY, "metrics", id],
    queryFn: () =>
      fetchJson<ExtensionMetrics>(
        client.starter,
        `/api/v1/extensions/${encodeURIComponent(id)}/metrics`,
      ),
    ...options,
  });
}

/** Cleanup dry-run — the purge manifest shown before confirming uninstall. */
export function useExtensionCleanupPreview(
  id: string,
  options?: ReadOptions<ExtensionCleanupPreview>,
): UseQueryResult<ExtensionCleanupPreview, StarterError> {
  const client = useRubixClient();
  return useQuery<ExtensionCleanupPreview, StarterError>({
    queryKey: [...EXTENSIONS_KEY, "cleanup", id],
    queryFn: () =>
      fetchJson<ExtensionCleanupPreview>(
        client.starter,
        `/api/v1/extensions/${encodeURIComponent(id)}/cleanup`,
      ),
    ...options,
  });
}

/** Uninstall + full data cleanup. `purge` defaults to `true` (the
 * comprehensive uninstall); pass `{ id, purge: false }` for the legacy
 * bundle-only removal. Invalidates the extensions cache on success. */
export interface ExtensionPurgeVars {
  id: string;
  purge?: boolean;
}

export function useExtensionPurge(
  options?: WriteOptions<ExtensionPurgeVars, ExtensionPurgeResult>,
): UseMutationResult<ExtensionPurgeResult, StarterError, ExtensionPurgeVars> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<ExtensionPurgeResult, StarterError, ExtensionPurgeVars>({
    mutationFn: ({ id, purge = true }) =>
      fetchJson<ExtensionPurgeResult>(
        client.starter,
        `/api/v1/extensions/${encodeURIComponent(id)}?purge=${purge}`,
        {
          method: "DELETE",
          headers: { ...readCsrfHeader() },
        },
      ),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: EXTENSIONS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
