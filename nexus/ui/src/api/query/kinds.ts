import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { QueryKindList } from "@/api/types";

// `GET /api/v1/query/kinds` — the catalogue of declarative query-kinds (WS-10)
// a panel can invoke by reverse-DNS name instead of pasting raw SQL. Read-only;
// the kind picker lists these and a kind-mode `QueryRequest` names one. The SQL
// stays server-side — only the descriptive surface (name, description,
// datasource shape, params schema) crosses the wire.
export function listQueryKinds(client: StarterClient): Promise<QueryKindList> {
  return fetchJson<QueryKindList>(client, `${client.apiPrefix}/query/kinds`, {
    method: "GET",
  });
}
