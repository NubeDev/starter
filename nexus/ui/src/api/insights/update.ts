import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { InsightSummary, UpdateInsightRequest } from "@/api/types";

// `PATCH /api/v1/insights/{id}` — update an insight's name, script, or
// params schema. Partial: omitted fields are left unchanged. The backend
// compile-checks a replaced script and returns 400 when it doesn't compile.
export function updateInsight(
  client: StarterClient,
  id: string,
  request: UpdateInsightRequest,
): Promise<InsightSummary> {
  return fetchJson<InsightSummary>(
    client,
    `${client.apiPrefix}/insights/${encodeURIComponent(id)}`,
    {
      method: "PATCH",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
