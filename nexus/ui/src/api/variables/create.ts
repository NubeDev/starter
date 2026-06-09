import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateVariableRequest, VariableDetail } from "@/api/types";

// `POST /api/v1/dashboards/{slug}/variables` — define a new variable on a
// dashboard. The name is unique per dashboard (409 on collision); the
// returned detail carries the server-assigned id.
export function createVariable(
  client: StarterClient,
  slug: string,
  request: CreateVariableRequest,
): Promise<VariableDetail> {
  return fetchJson<VariableDetail>(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}/variables`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
