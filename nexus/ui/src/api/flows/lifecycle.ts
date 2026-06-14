import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FlowDetail } from "@/api/types";

// `POST /api/v1/flows/{id}/start` — ask the FlowManager to run the flow on
// this node. Returns the flow with its updated `running` state (400 if the
// config is invalid).
export function startFlow(
  client: StarterClient,
  id: string,
): Promise<FlowDetail> {
  return fetchJson<FlowDetail>(
    client,
    `${client.apiPrefix}/flows/${encodeURIComponent(id)}/start`,
    { method: "POST", headers: readCsrfHeader() },
  );
}

// `POST /api/v1/flows/{id}/stop` — stop a running flow. Returns the flow
// with `running: false`.
export function stopFlow(
  client: StarterClient,
  id: string,
): Promise<FlowDetail> {
  return fetchJson<FlowDetail>(
    client,
    `${client.apiPrefix}/flows/${encodeURIComponent(id)}/stop`,
    { method: "POST", headers: readCsrfHeader() },
  );
}
