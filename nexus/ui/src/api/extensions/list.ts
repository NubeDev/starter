import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { ExtensionSummary } from "@/api/extensions/types";

// `GET /api/v1/extensions` — every installed extension with its lifecycle
// state, enablement and contribution counts (admin-gated server-side).
export function listExtensions(
  client: StarterClient,
): Promise<ExtensionSummary[]> {
  return fetchJson<ExtensionSummary[]>(
    client,
    `${client.apiPrefix}/extensions`,
  );
}
