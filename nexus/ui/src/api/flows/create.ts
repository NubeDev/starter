import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateFlowRequest, FlowDetail } from "@/api/types";

// `POST /api/v1/flows` — create a flow from its ArkFlow config (input /
// pipeline / output). The created flow is not started; use `startFlow`.
export function createFlow(
  client: StarterClient,
  request: CreateFlowRequest,
): Promise<FlowDetail> {
  return fetchJson<FlowDetail>(client, `${client.apiPrefix}/flows`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
