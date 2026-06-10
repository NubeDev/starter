import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FlowTableQueryRequest, QueryResponse } from "@/api/types";

// `POST /api/v1/flows/{id}/table/query` — query the table a flow's sink writes
// to, scoped to the flow (no datasource setup, no retyping the table name). The
// query runs read-only against the flow's sink connection. `{table}` in `sql`
// expands to the flow's configured table; omit `sql` for a recent-rows preview.
export function queryFlowTable(
  client: StarterClient,
  id: string,
  body: FlowTableQueryRequest,
): Promise<QueryResponse> {
  return fetchJson<QueryResponse>(
    client,
    `${client.apiPrefix}/flows/${encodeURIComponent(id)}/table/query`,
    { method: "POST", headers: readCsrfHeader(), body: JSON.stringify(body) },
  );
}
