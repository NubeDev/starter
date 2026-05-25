// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Narrow rewrite of the upstream `ui/src/api.ts`:
//
//   * `BASE_URL` now points at `/api/warehouse/ch` (the
//     `starter-warehouse` explorer sub-router) instead of
//     sql-studio's `/api`.
//   * `fetchOverview` is `GET /overview` rather than `GET /`
//     (we don't expose a root index).
//   * `fetchMetadata` and `sendShutdown` are removed — server
//     lifecycle lives in `starter-server` and is not part of the
//     explorer's surface.
//   * The `overview` schema drops `sqlite_version` and renames
//     `db_size` → `size_on_disk` to match the backend types
//     (see `crates/starter-warehouse/src/explorer/types.rs`).
//   * `fetchQuery` POSTs `{ sql: value }` to match the
//     `POST /query` handler in `crates/starter-warehouse/src/explorer/mod.rs`
//     (upstream sends `{ query: value }`).

import { z } from "zod";
import { createZodFetcher } from "zod-fetch";

const basePath = document.querySelector<HTMLMetaElement>(
  `meta[name="BASE_PATH"]`,
);
const API_ROOT = "/api/warehouse/ch";
const BASE_URL = import.meta.env.PROD
  ? basePath
    ? `${basePath.content}${API_ROOT}`
    : API_ROOT
  : `http://localhost:3030${API_ROOT}`;

const counts = z
  .object({
    name: z.string(),
    count: z.number(),
  })
  .array();

const overview = z.object({
  file_name: z.string(),
  size_on_disk: z.string(),
  created: z
    .string()
    .datetime()
    .transform((x) => new Date(x))
    .nullable(),
  modified: z
    .string()
    .datetime()
    .transform((x) => new Date(x))
    .nullable(),
  tables: z.number(),
  indexes: z.number(),
  triggers: z.number(),
  views: z.number(),
  row_counts: counts,
  column_counts: counts,
  index_counts: counts,
});

const tables = z.object({
  tables: counts,
});

const table = z.object({
  name: z.string(),
  sql: z.string().nullable(),
  row_count: z.number(),
  index_count: z.number(),
  column_count: z.number(),
  table_size: z.string(),
});

const tableData = z.object({
  columns: z.string().array(),
  rows: z.any().array().array(),
});

const query = z.object({
  columns: z.string().array(),
  rows: z.any().array().array(),
});

const autocomplete = z.object({
  tables: z
    .object({
      columns: z.string().array(),
      table_name: z.string(),
    })
    .array(),
});

const $fetch = createZodFetcher();

export const fetchOverview = () => $fetch(overview, `${BASE_URL}/overview`);
export const fetchTables = () => $fetch(tables, `${BASE_URL}/tables`);
export const fetchTable = (name: string) =>
  $fetch(table, `${BASE_URL}/tables/${name}`);
export const fetchTableData = (name: string, page: number) =>
  $fetch(tableData, `${BASE_URL}/tables/${name}/data?page=${page}`);
export const fetchQuery = (value: string) =>
  $fetch(query, `${BASE_URL}/query`, {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ sql: value }),
  });
export const fetchAutocomplete = () =>
  $fetch(autocomplete, `${BASE_URL}/autocomplete`);

const erdColumn = z.object({
  name: z.string(),
  data_type: z.string(),
  nullable: z.boolean(),
  is_primary_key: z.boolean(),
});

const erdTable = z.object({
  name: z.string(),
  columns: erdColumn.array(),
});

const erdRelationship = z.object({
  from_table: z.string(),
  from_column: z.string(),
  to_table: z.string(),
  to_column: z.string(),
});

const erd = z.object({
  tables: erdTable.array(),
  relationships: erdRelationship.array(),
});

export const fetchErd = () => $fetch(erd, `${BASE_URL}/erd`);

// ---------------------------------------------------------------
// PR 4 — rubix overlays. Wrappers for the broader warehouse REST
// surface (`starter-warehouse::rest::router`) and for the rubix
// verb dispatcher (`POST /api/v1/tools/{tool_id}`, served by
// `rubix-agent`).
//
// These live at a different mount path than the explorer routes
// above: the explorer is `/api/warehouse/ch/*`, the warehouse
// REST is `/api/warehouse/*` (no `ch`), and the rubix verbs are
// `/api/v1/tools/*`. Demos that only mount the explorer sub-router
// (e.g. `examples/ch-explorer`) will respond `404` to the calls
// below; the consuming components are written to treat `null` as
// "feature disabled" and render nothing rather than throwing.

const RUBIX_BASE = import.meta.env.PROD
  ? basePath
    ? `${basePath.content}/api`
    : "/api"
  : "http://localhost:3030/api";

const dictFreshness = z.object({
  status: z.enum([
    "ok",
    "stale",
    "refreshing",
    "failed_refresh",
    "never_refreshed",
  ]),
  last_successful_refresh: z.string().nullable().optional(),
  rows: z.number().optional(),
});

const warehouseStatus = z.object({
  dimensions: z.object({
    entities_dict: dictFreshness,
  }),
  ingest: z.object({
    async_insert_oldest_age_ms: z.number(),
    async_insert_backlog: z.number(),
  }),
});

export type WarehouseStatus = z.infer<typeof warehouseStatus>;

/// Hit `GET /api/warehouse/status`. Returns `null` when the
/// endpoint isn't mounted (HTTP 404) so callers can degrade
/// gracefully — the explorer-only demo binary doesn't carry the
/// W11 freshness probe.
export async function fetchWarehouseStatus(): Promise<WarehouseStatus | null> {
  const res = await fetch(`${RUBIX_BASE}/warehouse/status`, {
    headers: { Accept: "application/json" },
  });
  // `503` is W11's "dictionary failed_refresh" code; the body is
  // still a valid envelope so we surface it.
  if (res.status === 404) return null;
  if (!res.ok && res.status !== 503) {
    throw new Error(`warehouse status: HTTP ${res.status}`);
  }
  const body = await res.json();
  return warehouseStatus.parse(body);
}

// ---------------------------------------------------------------
// Rubix verb dispatcher. Every mutation flows through
// `POST /api/v1/tools/{tool_id}` on the rubix-agent so the
// snapshot-before-write + undo + changelog invariants are
// preserved. The explorer never bypasses this — anything
// destructive in the UI calls one of the wrappers below.
// ---------------------------------------------------------------

const RUBIX_VERB_BASE = import.meta.env.PROD
  ? basePath
    ? `${basePath.content}/api/v1/tools`
    : "/api/v1/tools"
  : "http://localhost:3030/api/v1/tools";

/// Sentinel returned by [`callRubixVerb`] when the dispatcher
/// itself isn't mounted (HTTP 404 on the tool id). Components use
/// this to disable their action buttons without crashing.
export const RUBIX_VERB_NOT_AVAILABLE = Symbol("RUBIX_VERB_NOT_AVAILABLE");

export type VerbOutcome<T> =
  | { ok: true; data: T }
  | { ok: false; status: number; error: string }
  | { ok: false; status: 404; error: typeof RUBIX_VERB_NOT_AVAILABLE };

export async function callRubixVerb<TIn, TOut>(
  toolId: string,
  body: TIn,
  responseSchema: z.ZodType<TOut>,
): Promise<VerbOutcome<TOut>> {
  const res = await fetch(`${RUBIX_VERB_BASE}/${toolId}`, {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (res.status === 404) {
    return { ok: false, status: 404, error: RUBIX_VERB_NOT_AVAILABLE };
  }
  if (!res.ok) {
    const errBody = await res.text();
    return { ok: false, status: res.status, error: errBody };
  }
  const parsed = await res.json();
  return { ok: true, data: responseSchema.parse(parsed) };
}

// `rubix.clickhouse.mart.{list,drop}` DTOs. Kept narrow — only
// the fields the UI actually consumes; the dispatcher tolerates
// extra fields on the wire.

const diagnostic = z
  .object({
    code: z.string(),
  })
  .passthrough();

const martSummary = z.object({
  mart_name: z.string(),
  ddl: z.string().nullable().optional(),
});

const martListResponse = z.object({
  summary: diagnostic,
  count: z.number(),
  marts: martSummary.array(),
});

const martDropResponse = z.object({
  summary: diagnostic,
  mart_name: z.string(),
  was_present: z.boolean(),
  dropped_at_ms: z.number(),
});

export type MartSummary = z.infer<typeof martSummary>;
export type MartListResponse = z.infer<typeof martListResponse>;
export type MartDropResponse = z.infer<typeof martDropResponse>;

export const callMartList = () =>
  callRubixVerb("rubix.clickhouse.mart.list", {}, martListResponse);

export const callMartDrop = (mart_name: string) =>
  callRubixVerb(
    "rubix.clickhouse.mart.drop",
    { mart_name },
    martDropResponse,
  );
