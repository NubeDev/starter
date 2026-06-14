import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { QueryHistoryList } from "@/api/types";

// `GET /api/v1/query-history` — the caller's recent query runs, newest (and
// starred) first. Powers the recall drawer in Explore so a user can re-run or
// pin a past query. RLS-scoped server-side: only this user's runs come back.
export function listQueryHistory(
  client: StarterClient,
): Promise<QueryHistoryList> {
  return fetchJson<QueryHistoryList>(
    client,
    `${client.apiPrefix}/query-history`,
  );
}
