import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { NodeTypeList } from "@/api/types";

// `GET /api/v1/flows/node-types` — the flow-builder palette: every node the
// engine can build, with its category and a JSON Schema for its config. Drives
// the palette and the schema-driven config form.
export function listNodeTypes(client: StarterClient): Promise<NodeTypeList> {
  return fetchJson<NodeTypeList>(client, `${client.apiPrefix}/flows/node-types`);
}
