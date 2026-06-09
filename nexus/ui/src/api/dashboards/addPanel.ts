import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreatePanelRequest, PanelDetail } from "@/api/types";

// `POST /api/v1/dashboards/{slug}/panels` — add a panel to a dashboard.
// The `layout` carries the grid position + field mapping (opaque to the
// backend); build the body with `widgetToCreatePanel`.
export function addPanel(
  client: StarterClient,
  slug: string,
  request: CreatePanelRequest,
): Promise<PanelDetail> {
  return fetchJson<PanelDetail>(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}/panels`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
