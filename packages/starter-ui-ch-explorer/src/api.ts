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
