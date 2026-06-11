import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { InsightSummary } from "@/api/types";

// `GET /api/v1/insights/{id}` — full detail for one insight, including its
// Rhai transform script.
export function getInsight(
  client: StarterClient,
  id: string,
): Promise<InsightSummary> {
  return fetchJson<InsightSummary>(
    client,
    `${client.apiPrefix}/insights/${encodeURIComponent(id)}`,
  );
}
