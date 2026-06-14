import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { MeResponse } from "@/api/types";

// `GET /api/v1/me` — the Nexus principal (subject, role, scopes, teams,
// tenant). Distinct from starter's `/auth/me`: Nexus surfaces tenant +
// teams + scopes so the SPA can gate per-user (usePrincipal/useCan).
// Cookie session auth is handled by `fetchJson` (credentials: include).
export function getMe(client: StarterClient): Promise<MeResponse> {
  return fetchJson<MeResponse>(client, `${client.apiPrefix}/me`);
}
