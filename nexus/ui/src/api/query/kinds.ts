import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type {
  CreateQueryKindRequest,
  QueryKindDetail,
  QueryKindList,
} from "@/api/types";

// Re-exported so callers (e.g. the Save-as-kind dialog) can import the request
// and detail shapes alongside the client function from one module.
export type { CreateQueryKindRequest, QueryKindDetail };

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

// `POST /api/v1/query-kinds` (hyphen, NOT `/query/kinds`) — promote raw SQL the
// user just authored into a reusable, server-validated query-kind. The server
// LINTS the SQL on save: if `tables` is non-empty the SQL must filter by
// `$caller_tenant_id` (else 400), and any `$param` referenced must be declared
// in `params_schema` (else 400); a duplicate `name` returns 409. Those messages
// surface verbatim via the thrown `StarterError.message`.
export function createQueryKind(
  client: StarterClient,
  body: CreateQueryKindRequest,
): Promise<QueryKindDetail> {
  return fetchJson<QueryKindDetail>(client, `${client.apiPrefix}/query-kinds`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(body),
  });
}
