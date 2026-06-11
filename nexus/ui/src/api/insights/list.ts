import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { InsightSummary } from "@/api/types";

// `GET /api/v1/insights` — the caller's tenant-scoped insights, as
// summaries (id, name, script). Powers the insights list and the picker
// for applying a transform to query results.
export function listInsights(
  client: StarterClient,
): Promise<InsightSummary[]> {
  return fetchJson<InsightSummary[]>(client, `${client.apiPrefix}/insights`);
}
