import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { QueryRequest, QueryResponse } from "@/api/types";

// `POST /api/v1/datasources/{id}/query` — run SQL against a *specific*
// datasource. This is what a panel uses, since a dashboard's panels can
// each target a different datasource. (The unscoped `POST /query` runs
// against the server-configured default; prefer this when an id is known.)
// Every safety bound is still applied server-side (R4).
export function queryDatasource(
  client: StarterClient,
  datasourceId: string,
  request: QueryRequest,
): Promise<QueryResponse> {
  return fetchJson<QueryResponse>(
    client,
    `${client.apiPrefix}/datasources/${encodeURIComponent(datasourceId)}/query`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
