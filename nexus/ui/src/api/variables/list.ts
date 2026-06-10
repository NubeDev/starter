import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { VariableDetail } from "@/api/types";

// `GET /api/v1/dashboards/{slug}/variables` — the dashboard's variable
// definitions, in `sort_order`. Powers the variable bar and the editor
// list. Tenant-scoped server-side (RLS); the caller passes only the slug.
export function listVariables(
  client: StarterClient,
  slug: string,
): Promise<VariableDetail[]> {
  return fetchJson<VariableDetail[]>(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}/variables`,
  );
}
