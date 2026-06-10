import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateInsightRequest, InsightSummary } from "@/api/types";

// `POST /api/v1/insights` — register a reusable Rhai transform under the
// caller's tenant. The backend compile-checks the script and returns 400
// with a message when it doesn't compile.
export function createInsight(
  client: StarterClient,
  request: CreateInsightRequest,
): Promise<InsightSummary> {
  return fetchJson<InsightSummary>(client, `${client.apiPrefix}/insights`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
