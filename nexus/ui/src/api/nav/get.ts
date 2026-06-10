import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { NavNodeDetail } from "@/api/types";

// `GET /api/v1/nav/{id}` — one nav node, gated on `view` of the node itself
// (WS-13 §6). A page opened under `?nav=:id` loads its node here to merge the
// node's context into the PageContext.
export function getNavNode(
  client: StarterClient,
  id: string,
): Promise<NavNodeDetail> {
  return fetchJson<NavNodeDetail>(
    client,
    `${client.apiPrefix}/nav/${encodeURIComponent(id)}`,
  );
}
