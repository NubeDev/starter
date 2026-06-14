import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FlowDebugEnableResponse, FlowDebugStatus } from "@/api/types";

// `POST /api/v1/flows/{id}/debug/enable` — turn on per-node value/sample
// capture for a running flow and mint the short-lived token + SSE URL the
// `EventSource` opens. Never restarts the flow.
export function enableFlowDebug(
  client: StarterClient,
  id: string,
): Promise<FlowDebugEnableResponse> {
  return fetchJson<FlowDebugEnableResponse>(
    client,
    `${client.apiPrefix}/flows/${encodeURIComponent(id)}/debug/enable`,
    { method: "POST", headers: readCsrfHeader() },
  );
}

// `POST /api/v1/flows/{id}/debug/disable` — stop capture so the flow stops
// sampling rows (the taps stay installed but go quiet).
export function disableFlowDebug(
  client: StarterClient,
  id: string,
): Promise<FlowDebugStatus> {
  return fetchJson<FlowDebugStatus>(
    client,
    `${client.apiPrefix}/flows/${encodeURIComponent(id)}/debug/disable`,
    { method: "POST", headers: readCsrfHeader() },
  );
}
