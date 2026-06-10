import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { ChangePage } from "@/api/types";

// `GET /api/v1/audit/resources/{kind}/{id}` — one resource's change history,
// newest first. Powers a "History" tab on a dashboard/datasource. The resource
// is pinned by the path; only paging/time-window are caller-supplied.
export interface ResourceHistoryParams {
  since?: string;
  until?: string;
  limit?: number;
  cursor?: string;
}

export function resourceHistory(
  client: StarterClient,
  kind: string,
  id: string,
  params: ResourceHistoryParams = {},
): Promise<ChangePage> {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== "") {
      search.set(key, String(value));
    }
  }
  const query = search.toString();
  const suffix = query ? `?${query}` : "";
  return fetchJson<ChangePage>(
    client,
    `${client.apiPrefix}/audit/resources/${encodeURIComponent(kind)}/${encodeURIComponent(id)}${suffix}`,
  );
}
