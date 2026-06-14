import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DatasourceSchema, QueryResponse } from "@/api/types";

// Sentinel datasource id used in the UI to mean "the nexus control-plane DB"
// rather than a registered datasource. It is NOT a real datasource UUID — the
// metadata pool lives behind its own endpoint (`POST /nexus-db/query`), so any
// code resolving a datasourceId must special-case this before hitting
// `/datasources/:id/query`. Kept here so producer (picker) and consumer
// (widget query) agree on the exact string.
export const NEXUS_DB_DATASOURCE_ID = "nexus-db";

// `POST /api/v1/nexus-db/query` — a read-only window into the control-plane
// DB itself (`state.metadata`: users, datasources, dashboards, flows, agents).
// Unlike `/datasources/:id/query` this takes only raw `{ sql }` — no
// federation, no kind, no time-range/variable macro expansion — and is
// admin-only + tenant-RLS-scoped + read-only server-side. The response is the
// same `{ columns, rows, stats }` shape the result grid renders, so callers
// can treat it interchangeably with a datasource query result.
export function queryNexusDb(
  client: StarterClient,
  sql: string,
): Promise<QueryResponse> {
  return fetchJson<QueryResponse>(client, `${client.apiPrefix}/nexus-db/query`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify({ sql }),
  });
}

// `GET /api/v1/nexus-db/schema` — the control-plane DB's tables, columns, and
// foreign keys, for the schema (ER) diagram. Same `DatasourceSchema` shape the
// datasource schema route returns, so the diagram renders the metadata DB
// exactly like any datasource. Admin-only + tenant-scoped server-side.
export function getNexusDbSchema(
  client: StarterClient,
): Promise<DatasourceSchema> {
  return fetchJson<DatasourceSchema>(
    client,
    `${client.apiPrefix}/nexus-db/schema`,
  );
}
