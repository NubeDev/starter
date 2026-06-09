import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { QueryRequest, QueryResponse } from "@/api/types";

// `POST /api/v1/query` — run a panel's SQL against its datasource and get
// back `{ columns, rows, stats }`. The SQL is pushed down to the source
// database (WHERE/LIMIT execute there); the response is capped server-
// side (`stats.truncated` signals a hit cap). This is the one data path
// every panel renders from (F0/F6).
export function runQuery(
  client: StarterClient,
  request: QueryRequest,
): Promise<QueryResponse> {
  return fetchJson<QueryResponse>(client, `${client.apiPrefix}/query`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
