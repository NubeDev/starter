import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type {
  DatasourceKindList,
  DatasourceKindSummary,
} from "@/api/types";

// `GET /api/v1/datasources/kinds` — the catalogue of registered connector kinds
// (postgres, mqtt, zenoh). Each entry carries the JSON Schema the create form
// renders its config fields from, which of those fields are secrets, and how the
// kind is probed. Static for a deployment, so the hook caches it indefinitely.
// Returns the unwrapped `kinds` array (the wire envelope is `{ kinds: [...] }`).
export function listDatasourceKinds(
  client: StarterClient,
): Promise<DatasourceKindSummary[]> {
  return fetchJson<DatasourceKindList>(
    client,
    `${client.apiPrefix}/datasources/kinds`,
  ).then((list) => list.kinds);
}
