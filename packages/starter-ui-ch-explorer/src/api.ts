// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Zod shapes + typed fetchers for `GET /api/warehouse/ch/*`, the
// explorer sub-router served by `starter-warehouse`. Every fetcher
// goes through `fetchJson(starter, …)` from
// `@nube/starter-client-ts`, so the library never opens a raw
// `fetch` — the host's `StarterClient` owns base URL, credentials,
// and error envelope handling.
//
// History:
//   * `BASE_URL` was sql-studio's `/api`; now resolved relative to
//     the host's `StarterClient.baseUrl` as `/api/warehouse/ch/*`.
//   * `fetchOverview` is `GET /overview` (was `GET /`).
//   * `fetchMetadata` and `sendShutdown` are removed — lifecycle
//     lives in `starter-server`.
//   * `overview` drops `sqlite_version` and renames `db_size`
//     → `size_on_disk` to match
//     `starter-warehouse::explorer::types`.
//   * `fetchQuery` POSTs `{ sql: value }` (upstream sent `{ query }`).
//   * PR 2 (rubix shell integration): removed the ad-hoc
//     `RUBIX_VERB_*` transport and `fetchWarehouseStatus` wrapper.
//     Destructive surfaces flow through
//     `@nube/rubix-client-react` typed hooks (`./rubix/*`); the
//     warehouse status read lives in `./rubix/freshness-tiles.tsx`.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { z } from "zod";
import { fetchJson, type StarterClient } from "@nube/starter-client-ts";

const API_ROOT = "/api/warehouse/ch";

const counts = z
  .object({
    name: z.string(),
    count: z.number(),
  })
  .array();

export const overviewSchema = z.object({
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

export const tablesSchema = z.object({
  tables: counts,
});

export const tableSchema = z.object({
  name: z.string(),
  sql: z.string().nullable(),
  row_count: z.number(),
  index_count: z.number(),
  column_count: z.number(),
  table_size: z.string(),
});

export const tableDataSchema = z.object({
  columns: z.string().array(),
  rows: z.any().array().array(),
});

export const querySchema = z.object({
  columns: z.string().array(),
  rows: z.any().array().array(),
});

export const autocompleteSchema = z.object({
  tables: z
    .object({
      columns: z.string().array(),
      table_name: z.string(),
    })
    .array(),
});

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

export const erdSchema = z.object({
  tables: erdTable.array(),
  relationships: erdRelationship.array(),
});

export type Overview = z.infer<typeof overviewSchema>;
export type TablesList = z.infer<typeof tablesSchema>;
export type TableMeta = z.infer<typeof tableSchema>;
export type TableData = z.infer<typeof tableDataSchema>;
export type QueryResult = z.infer<typeof querySchema>;
export type Autocomplete = z.infer<typeof autocompleteSchema>;
export type Erd = z.infer<typeof erdSchema>;

async function getValidated<T>(
  starter: StarterClient,
  path: string,
  schema: z.ZodType<T>,
): Promise<T> {
  const body = await fetchJson<unknown>(starter, path);
  return schema.parse(body);
}

export const fetchOverview = (starter: StarterClient): Promise<Overview> =>
  getValidated(starter, `${API_ROOT}/overview`, overviewSchema);

export const fetchTables = (starter: StarterClient): Promise<TablesList> =>
  getValidated(starter, `${API_ROOT}/tables`, tablesSchema);

export const fetchTable = (
  starter: StarterClient,
  name: string,
): Promise<TableMeta> =>
  getValidated(
    starter,
    `${API_ROOT}/tables/${encodeURIComponent(name)}`,
    tableSchema,
  );

export const fetchTableData = (
  starter: StarterClient,
  name: string,
  page: number,
): Promise<TableData> =>
  getValidated(
    starter,
    `${API_ROOT}/tables/${encodeURIComponent(name)}/data?page=${page}`,
    tableDataSchema,
  );

export async function fetchQuery(
  starter: StarterClient,
  sql: string,
): Promise<QueryResult> {
  const body = await fetchJson<unknown>(starter, `${API_ROOT}/query`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ sql }),
  });
  return querySchema.parse(body);
}

export const fetchAutocomplete = (
  starter: StarterClient,
): Promise<Autocomplete> =>
  getValidated(starter, `${API_ROOT}/autocomplete`, autocompleteSchema);

export const fetchErd = (starter: StarterClient): Promise<Erd> =>
  getValidated(starter, `${API_ROOT}/erd`, erdSchema);
