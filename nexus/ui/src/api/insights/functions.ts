import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { InsightFunctionCatalog, InsightFunctionDoc } from "@/api/types";

// `GET /api/v1/insights/functions` — the curated function surface the sandbox
// exposes, as display docs (name, signature, summary, category, example). The
// Workbench renders these as a cheatsheet AND feeds them to the editor's
// autocomplete. Unwraps the `{ functions }` envelope to the flat list callers
// actually iterate.
export async function listInsightFunctions(
  client: StarterClient,
): Promise<InsightFunctionDoc[]> {
  const catalog = await fetchJson<InsightFunctionCatalog>(
    client,
    `${client.apiPrefix}/insights/functions`,
  );
  return catalog.functions;
}
