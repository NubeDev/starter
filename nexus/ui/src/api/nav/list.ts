import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { NavNodeDetail } from "@/api/types";

// `GET /api/v1/nav` — the caller's navigation tree, already access-filtered to
// the nodes they hold `view` on (WS-13 §6). Returns a flat list; the tree is
// rebuilt client-side from `parent_id` + `sort_order`.
export function listNav(client: StarterClient): Promise<NavNodeDetail[]> {
  return fetchJson<NavNodeDetail[]>(client, `${client.apiPrefix}/nav`);
}
