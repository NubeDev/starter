import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DashboardSummary } from "@/api/types";

// `GET /api/v1/dashboards` — the caller's tenant-scoped dashboards as
// summaries (id, name, slug). Powers the sidebar list.
export function listDashboards(
  client: StarterClient,
): Promise<DashboardSummary[]> {
  return fetchJson<DashboardSummary[]>(client, `${client.apiPrefix}/dashboards`);
}
