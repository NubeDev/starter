import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateDashboardRequest, DashboardSummary } from "@/api/types";

// `POST /api/v1/dashboards` — create a dashboard (409 if the slug is
// taken). Returns the summary; the caller navigates to its slug.
export function createDashboard(
  client: StarterClient,
  request: CreateDashboardRequest,
): Promise<DashboardSummary> {
  return fetchJson<DashboardSummary>(client, `${client.apiPrefix}/dashboards`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
