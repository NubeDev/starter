import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { ChangePage } from "@/api/types";

// `GET /api/v1/audit` — filtered, paged, newest-first audit log for the
// caller's tenant (admin-gated server-side). The audit screen and any
// per-resource History tab read through this; pagination is the opaque
// `cursor` echoed back in `next_cursor`.
export interface AuditFilter {
  actor_kind?: string;
  actor_id?: string;
  actor_model?: string;
  resource_kind?: string;
  resource_id?: string;
  group_id?: string;
  since?: string;
  until?: string;
  limit?: number;
  cursor?: string;
}

export function listAudit(
  client: StarterClient,
  filter: AuditFilter = {},
): Promise<ChangePage> {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filter)) {
    if (value !== undefined && value !== null && value !== "") {
      params.set(key, String(value));
    }
  }
  const query = params.toString();
  const suffix = query ? `?${query}` : "";
  return fetchJson<ChangePage>(client, `${client.apiPrefix}/audit${suffix}`);
}
