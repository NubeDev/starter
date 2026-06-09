import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

// `GET /api/v1/tags/keys` — the distinct tag keys in use across the tenant,
// for key autocomplete in the tag editor.
export function listTagKeys(client: StarterClient): Promise<string[]> {
  return fetchJson<string[]>(client, `${client.apiPrefix}/tags/keys`);
}
