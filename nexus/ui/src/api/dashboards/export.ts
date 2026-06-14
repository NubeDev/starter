import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DashboardExport, DashboardSummary } from "@/api/types";

// `GET /api/v1/dashboards/{slug}/export` — the portable dashboard JSON model
// (appearance + panels + variables). The caller serialises it to a file.
export function exportDashboard(
  client: StarterClient,
  slug: string,
): Promise<DashboardExport> {
  return fetchJson<DashboardExport>(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}/export`,
  );
}

// `POST /api/v1/dashboards/import` — re-create a dashboard from a previously
// exported model (400 on an unknown `schema_version`, 409 on a slug clash).
export function importDashboard(
  client: StarterClient,
  model: DashboardExport,
): Promise<DashboardSummary> {
  return fetchJson<DashboardSummary>(
    client,
    `${client.apiPrefix}/dashboards/import`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(model),
    },
  );
}

// `POST /api/v1/dashboards/{slug}/duplicate` — copy a dashboard with its panels
// and variables under a fresh id and a derived slug. Returns the new summary.
export function duplicateDashboard(
  client: StarterClient,
  slug: string,
): Promise<DashboardSummary> {
  return fetchJson<DashboardSummary>(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}/duplicate`,
    { method: "POST", headers: readCsrfHeader() },
  );
}
