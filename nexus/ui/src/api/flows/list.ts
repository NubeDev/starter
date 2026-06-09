import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FlowSummary } from "@/api/types";

// `GET /api/v1/flows` — the tenant's saved flows (long-running ArkFlow
// ingestion pipelines), as summaries with their enabled/running state.
export function listFlows(client: StarterClient): Promise<FlowSummary[]> {
  return fetchJson<FlowSummary[]>(client, `${client.apiPrefix}/flows`);
}
