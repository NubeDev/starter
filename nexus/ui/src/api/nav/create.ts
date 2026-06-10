import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateNavNodeRequest, NavNodeDetail } from "@/api/types";

// `POST /api/v1/nav` — create a nav node. A `dashboard` target is validated to
// exist in the caller's tenant (400 otherwise). Omit `target` for a `group`
// header. Context only applies to a `dashboard` target.
export function createNavNode(
  client: StarterClient,
  request: CreateNavNodeRequest,
): Promise<NavNodeDetail> {
  return fetchJson<NavNodeDetail>(client, `${client.apiPrefix}/nav`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
