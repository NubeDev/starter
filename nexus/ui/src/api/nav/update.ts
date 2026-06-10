import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { NavNodeDetail, UpdateNavNodeRequest } from "@/api/types";

// `PATCH /api/v1/nav/{id}` — retitle / reparent / reorder / retarget a node
// (WS-13 §4). Partial: omitted fields are unchanged. The `clear_*` flags drop a
// field (e.g. `clear_context` when retargeting a dashboard mount to a group).
export function updateNavNode(
  client: StarterClient,
  id: string,
  request: UpdateNavNodeRequest,
): Promise<NavNodeDetail> {
  return fetchJson<NavNodeDetail>(
    client,
    `${client.apiPrefix}/nav/${encodeURIComponent(id)}`,
    {
      method: "PATCH",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
