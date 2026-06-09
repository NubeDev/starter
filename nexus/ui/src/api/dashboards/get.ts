import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DashboardDetail } from "@/api/types";

// `GET /api/v1/dashboards/{slug}` — one dashboard with its panels. The
// caller adapts each `PanelDetail` to a `Widget` via `panelToWidget`.
export function getDashboard(
  client: StarterClient,
  slug: string,
): Promise<DashboardDetail> {
  return fetchJson<DashboardDetail>(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}`,
  );
}
