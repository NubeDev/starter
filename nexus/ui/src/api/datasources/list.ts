import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DatasourceSummary } from "@/api/types";

// `GET /api/v1/datasources` — the caller's tenant-scoped datasources, as
// summaries (id, name, kind). Powers the datasource picker in the query
// editor and panel config.
export function listDatasources(
  client: StarterClient,
): Promise<DatasourceSummary[]> {
  return fetchJson<DatasourceSummary[]>(client, `${client.apiPrefix}/datasources`);
}
