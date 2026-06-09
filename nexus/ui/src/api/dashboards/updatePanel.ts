import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { PanelDetail, UpdatePanelRequest } from "@/api/types";

// `PATCH /api/v1/panels/{id}` — partial update of a panel. Any subset of
// `layout` / `title` / `sql` / `datasource_id` / `viz`; omitted fields are
// left unchanged. The canvas uses it to persist a moved/resized panel by
// sending only the new `layout` (the opaque grid JSON it owns).
export function updatePanel(
  client: StarterClient,
  panelId: string,
  request: UpdatePanelRequest,
): Promise<PanelDetail> {
  return fetchJson<PanelDetail>(
    client,
    `${client.apiPrefix}/panels/${encodeURIComponent(panelId)}`,
    {
      method: "PATCH",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
