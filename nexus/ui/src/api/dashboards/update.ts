import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DashboardSummary, UpdateDashboardRequest } from "@/api/types";

// `PATCH /api/v1/dashboards/{slug}` — rename or re-slug a dashboard (409 if the
// new slug is taken). Re-slugging changes only the route alias; the immutable
// id is unchanged, so grants and panel refs stay valid. Returns the updated
// summary so the caller can navigate to a new slug.
export function updateDashboard(
  client: StarterClient,
  slug: string,
  request: UpdateDashboardRequest,
): Promise<DashboardSummary> {
  return fetchJson<DashboardSummary>(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}`,
    {
      method: "PATCH",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
